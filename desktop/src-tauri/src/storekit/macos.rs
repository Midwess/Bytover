#![allow(deprecated)]

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use dispatch::Queue;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, AnyThread, DefinedClass};
use objc2_foundation::{NSArray, NSError, NSObjectProtocol, NSSet, NSString};
use objc2_store_kit::{
    SKErrorCode, SKMutablePayment, SKPaymentQueue, SKPaymentTransaction,
    SKPaymentTransactionObserver, SKPaymentTransactionState, SKProductsRequest,
    SKProductsRequestDelegate, SKProductsResponse, SKRequest, SKRequestDelegate,
};
use tokio::sync::oneshot;

use super::{ProductAvailabilityReport, StoreKitClient, StoreKitError, StoreKitTransaction};

type TxResult = Result<StoreKitTransaction, StoreKitError>;
type RestoreResult = Result<Vec<StoreKitTransaction>, StoreKitError>;
type ProductsResult = Result<ProductAvailabilityReport, StoreKitError>;

const PRODUCTS_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Default)]
struct ObserverState {
    pending_by_product: HashMap<String, VecDeque<oneshot::Sender<TxResult>>>,
    restore_waiters: VecDeque<oneshot::Sender<RestoreResult>>,
    restore_accumulator: Vec<StoreKitTransaction>,
    restore_in_progress: bool,
    orphans: Vec<StoreKitTransaction>,
}

impl ObserverState {
    fn push_purchase_waiter(&mut self, product_id: &str, sender: oneshot::Sender<TxResult>) {
        self.pending_by_product
            .entry(product_id.to_owned())
            .or_default()
            .push_back(sender);
    }

    fn push_restore_waiter(&mut self, sender: oneshot::Sender<RestoreResult>) {
        self.restore_waiters.push_back(sender);
        self.restore_in_progress = true;
        self.restore_accumulator.clear();
    }

    fn route_success(&mut self, mapped: StoreKitTransaction, is_restored: bool) {
        if is_restored && self.restore_in_progress {
            self.restore_accumulator.push(mapped);
            return;
        }
        if let Some(queue) = self.pending_by_product.get_mut(&mapped.product_id) {
            if let Some(sender) = queue.pop_front() {
                let _ = sender.send(Ok(mapped));
                return;
            }
        }
        self.orphans.push(mapped);
    }

    fn route_failure(&mut self, product_id: &str, err: StoreKitError) {
        if let Some(queue) = self.pending_by_product.get_mut(product_id) {
            if let Some(sender) = queue.pop_front() {
                let _ = sender.send(Err(err));
                return;
            }
        }
        log::warn!("[storekit] transaction failure for {product_id} with no waiter: {err}");
    }

    fn finish_restore_success(&mut self) {
        let batch = std::mem::take(&mut self.restore_accumulator);
        self.restore_in_progress = false;
        while let Some(sender) = self.restore_waiters.pop_front() {
            let _ = sender.send(Ok(batch.clone()));
        }
    }

    fn finish_restore_failure(&mut self, err: StoreKitError) {
        self.restore_accumulator.clear();
        self.restore_in_progress = false;
        while let Some(sender) = self.restore_waiters.pop_front() {
            let _ = sender.send(Err(StoreKitError::Failed(err.to_string())));
        }
    }

    fn take_orphans(&mut self) -> Vec<StoreKitTransaction> {
        std::mem::take(&mut self.orphans)
    }
}

struct ObserverIvars {
    state: Arc<Mutex<ObserverState>>,
}

define_class!(
    #[unsafe(super(objc2_foundation::NSObject))]
    #[name = "BytoverTxObserver"]
    #[ivars = ObserverIvars]
    struct BytoverTxObserver;

    unsafe impl NSObjectProtocol for BytoverTxObserver {}

    unsafe impl SKPaymentTransactionObserver for BytoverTxObserver {
        #[unsafe(method(paymentQueue:updatedTransactions:))]
        fn payment_queue_updated_transactions(
            &self,
            queue: &SKPaymentQueue,
            transactions: &NSArray<SKPaymentTransaction>,
        ) {
            process_updated_transactions(self.ivars().state.clone(), queue, transactions);
        }

        #[unsafe(method(paymentQueue:restoreCompletedTransactionsFailedWithError:))]
        fn payment_queue_restore_failed(
            &self,
            _queue: &SKPaymentQueue,
            error: &NSError,
        ) {
            let msg = error.localizedDescription().to_string();
            let mut guard = self.ivars().state.lock().expect("observer state poisoned");
            guard.finish_restore_failure(StoreKitError::Failed(msg));
        }

        #[unsafe(method(paymentQueueRestoreCompletedTransactionsFinished:))]
        fn payment_queue_restore_finished(&self, _queue: &SKPaymentQueue) {
            let mut guard = self.ivars().state.lock().expect("observer state poisoned");
            guard.finish_restore_success();
        }
    }
);

impl BytoverTxObserver {
    fn new(state: Arc<Mutex<ObserverState>>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(ObserverIvars { state });
        unsafe { msg_send![super(this), init] }
    }
}

fn process_updated_transactions(
    state: Arc<Mutex<ObserverState>>,
    queue: &SKPaymentQueue,
    transactions: &NSArray<SKPaymentTransaction>,
) {
    let len = transactions.count();
    for i in 0..len {
        let tx = unsafe { transactions.objectAtIndex_unchecked(i) };
        let tx_state = unsafe { tx.transactionState() };
        let raw = tx_state.0;
        if raw == SKPaymentTransactionState::Purchased.0 {
            if let Some(mapped) = map_transaction(tx) {
                let mut guard = state.lock().expect("observer state poisoned");
                guard.route_success(mapped, false);
            }
        } else if raw == SKPaymentTransactionState::Restored.0 {
            if let Some(mapped) = map_transaction(tx) {
                let mut guard = state.lock().expect("observer state poisoned");
                guard.route_success(mapped, true);
            }
        } else if raw == SKPaymentTransactionState::Failed.0 {
            let payment = unsafe { tx.payment() };
            let product_id = unsafe { payment.productIdentifier() }.to_string();
            let err = extract_transaction_error(tx);
            {
                let mut guard = state.lock().expect("observer state poisoned");
                guard.route_failure(&product_id, err);
            }
            unsafe { queue.finishTransaction(tx) };
        }
    }
}

fn extract_transaction_error(tx: &SKPaymentTransaction) -> StoreKitError {
    let error = unsafe { tx.error() };
    match error {
        Some(err) => {
            let code = err.code();
            let msg = err.localizedDescription().to_string();
            if code == SKErrorCode::PaymentCancelled.0 {
                StoreKitError::UserCancelled
            } else {
                StoreKitError::Failed(msg)
            }
        }
        None => StoreKitError::Failed("StoreKit transaction failed with no error info".to_owned()),
    }
}

struct ProductsIvars {
    sender: Mutex<Option<oneshot::Sender<ProductsResult>>>,
}

define_class!(
    #[unsafe(super(objc2_foundation::NSObject))]
    #[name = "BytoverProductsDelegate"]
    #[ivars = ProductsIvars]
    struct BytoverProductsDelegate;

    unsafe impl NSObjectProtocol for BytoverProductsDelegate {}

    unsafe impl SKRequestDelegate for BytoverProductsDelegate {
        #[unsafe(method(request:didFailWithError:))]
        fn request_did_fail_with_error(&self, _request: &SKRequest, error: &NSError) {
            let desc = error.localizedDescription().to_string();
            if let Some(sender) = take_products_sender(&self.ivars().sender) {
                let _ = sender.send(Ok(ProductAvailabilityReport {
                    available: Vec::new(),
                    invalid: Vec::new(),
                    error: Some(desc),
                }));
            }
        }
    }

    unsafe impl SKProductsRequestDelegate for BytoverProductsDelegate {
        #[unsafe(method(productsRequest:didReceiveResponse:))]
        fn products_request_did_receive_response(
            &self,
            _request: &SKProductsRequest,
            response: &SKProductsResponse,
        ) {
            let products = unsafe { response.products() };
            let invalid_ids = unsafe { response.invalidProductIdentifiers() };
            let available = collect_product_ids(&products);
            let invalid = collect_ns_strings(&invalid_ids);
            if let Some(sender) = take_products_sender(&self.ivars().sender) {
                let _ = sender.send(Ok(ProductAvailabilityReport {
                    available,
                    invalid,
                    error: None,
                }));
            }
        }
    }
);

impl BytoverProductsDelegate {
    fn new(sender: oneshot::Sender<ProductsResult>) -> Retained<Self> {
        let ivars = ProductsIvars {
            sender: Mutex::new(Some(sender)),
        };
        let this = Self::alloc().set_ivars(ivars);
        unsafe { msg_send![super(this), init] }
    }
}

fn take_products_sender(
    slot: &Mutex<Option<oneshot::Sender<ProductsResult>>>,
) -> Option<oneshot::Sender<ProductsResult>> {
    slot.lock().ok().and_then(|mut guard| guard.take())
}

fn collect_product_ids(products: &NSArray<objc2_store_kit::SKProduct>) -> Vec<String> {
    let len = products.count();
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let p = unsafe { products.objectAtIndex_unchecked(i) };
        out.push(unsafe { p.productIdentifier() }.to_string());
    }
    out
}

fn collect_ns_strings(arr: &NSArray<NSString>) -> Vec<String> {
    let len = arr.count();
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let s = unsafe { arr.objectAtIndex_unchecked(i) };
        out.push(s.to_string());
    }
    out
}

fn build_product_id_set(product_ids: &[&str]) -> Retained<NSSet<NSString>> {
    let strings: Vec<Retained<NSString>> =
        product_ids.iter().map(|id| NSString::from_str(id)).collect();
    NSSet::from_retained_slice(&strings)
}

struct ObserverHandle {
    state: Arc<Mutex<ObserverState>>,
}

static OBSERVER_HANDLE: OnceLock<ObserverHandle> = OnceLock::new();

impl ObserverHandle {
    fn shared() -> &'static ObserverHandle {
        OBSERVER_HANDLE.get_or_init(|| {
            let state = Arc::new(Mutex::new(ObserverState::default()));
            let state_for_closure = state.clone();
            Queue::main().exec_sync(move || {
                let observer = BytoverTxObserver::new(state_for_closure);
                let proto: &ProtocolObject<dyn SKPaymentTransactionObserver> =
                    ProtocolObject::from_ref(&*observer);
                unsafe {
                    SKPaymentQueue::defaultQueue().addTransactionObserver(proto);
                }
                std::mem::forget(observer);
            });
            log::info!("[storekit] SKPaymentTransactionObserver registered on main thread");
            ObserverHandle { state }
        })
    }

    fn register_purchase_waiter(&self, product_id: &str, sender: oneshot::Sender<TxResult>) {
        let mut guard = self.state.lock().expect("observer state poisoned");
        guard.push_purchase_waiter(product_id, sender);
    }

    fn register_restore_waiter(&self, sender: oneshot::Sender<RestoreResult>) {
        let mut guard = self.state.lock().expect("observer state poisoned");
        guard.push_restore_waiter(sender);
    }

    fn take_orphans(&self) -> Vec<StoreKitTransaction> {
        let mut guard = self.state.lock().expect("observer state poisoned");
        guard.take_orphans()
    }
}

pub struct MacStoreKitClient;

impl MacStoreKitClient {
    pub fn shared() -> Self {
        Self
    }

    fn default_queue() -> Retained<SKPaymentQueue> {
        unsafe { SKPaymentQueue::defaultQueue() }
    }

    fn can_make_payments() -> bool {
        unsafe { SKPaymentQueue::canMakePayments() }
    }

    fn read_queue_transactions() -> Vec<StoreKitTransaction> {
        let queue = Self::default_queue();
        let array = unsafe { queue.transactions() };
        let len = array.count();
        let mut out = Vec::with_capacity(len);
        for i in 0..len {
            let tx = unsafe { array.objectAtIndex_unchecked(i) };
            if let Some(mapped) = map_transaction(tx) {
                out.push(mapped);
            }
        }
        out
    }

    fn finish_transaction(transaction_id: &str) -> Result<(), StoreKitError> {
        let queue = Self::default_queue();
        let array = unsafe { queue.transactions() };
        let len = array.count();
        for i in 0..len {
            let tx = unsafe { array.objectAtIndex_unchecked(i) };
            if let Some(tx_id) = unsafe { tx.transactionIdentifier() } {
                if tx_id.to_string() == transaction_id {
                    unsafe { queue.finishTransaction(tx) };
                    return Ok(());
                }
            }
        }
        Err(StoreKitError::Failed(format!(
            "no pending transaction with id {transaction_id} to finish"
        )))
    }

    fn enqueue_payment(product_id: String) {
        Queue::main().exec_sync(move || unsafe {
            let payment = SKMutablePayment::new();
            let ns_id = NSString::from_str(&product_id);
            payment.setProductIdentifier(&ns_id);
            SKPaymentQueue::defaultQueue().addPayment(&payment);
        });
    }

    fn start_restore() {
        Queue::main().exec_sync(|| unsafe {
            SKPaymentQueue::defaultQueue().restoreCompletedTransactions();
        });
    }

    fn start_products_request(
        product_ids: Vec<String>,
    ) -> oneshot::Receiver<ProductsResult> {
        let (tx, rx) = oneshot::channel();
        Queue::main().exec_sync(move || {
            let delegate = BytoverProductsDelegate::new(tx);
            let refs: Vec<&str> = product_ids.iter().map(|s| s.as_str()).collect();
            let id_set = build_product_id_set(&refs);
            let request = unsafe {
                SKProductsRequest::initWithProductIdentifiers(
                    SKProductsRequest::alloc(),
                    &id_set,
                )
            };
            let proto: &ProtocolObject<dyn SKProductsRequestDelegate> =
                ProtocolObject::from_ref(&*delegate);
            unsafe {
                request.setDelegate(Some(proto));
                request.start();
            }
            std::mem::forget(delegate);
            std::mem::forget(request);
        });
        rx
    }
}

fn map_transaction(tx: &SKPaymentTransaction) -> Option<StoreKitTransaction> {
    let state = unsafe { tx.transactionState() };
    let is_done = state.0 == SKPaymentTransactionState::Purchased.0
        || state.0 == SKPaymentTransactionState::Restored.0;
    if !is_done {
        return None;
    }
    let transaction_id = unsafe { tx.transactionIdentifier() }.map(|s| s.to_string())?;
    let payment = unsafe { tx.payment() };
    let product_id = unsafe { payment.productIdentifier() }.to_string();
    let original_transaction_id = unsafe { tx.originalTransaction() }
        .and_then(|o| unsafe { o.transactionIdentifier() })
        .map(|s| s.to_string());

    Some(StoreKitTransaction {
        transaction_id,
        product_id,
        original_transaction_id,
    })
}

#[async_trait]
impl StoreKitClient for MacStoreKitClient {
    async fn purchase(&self, product_id: &str) -> Result<StoreKitTransaction, StoreKitError> {
        if !Self::can_make_payments() {
            return Err(StoreKitError::Failed(
                "canMakePayments returned false — in-app purchases are disabled on this device"
                    .to_owned(),
            ));
        }
        let handle = ObserverHandle::shared();
        let (tx, rx) = oneshot::channel();
        handle.register_purchase_waiter(product_id, tx);

        let product_id_owned = product_id.to_owned();
        tokio::task::spawn_blocking(move || Self::enqueue_payment(product_id_owned))
            .await
            .map_err(|e| StoreKitError::Failed(e.to_string()))?;

        rx.await.map_err(|_| StoreKitError::ChannelClosed)?
    }

    async fn unfinished_transactions(&self) -> Result<Vec<StoreKitTransaction>, StoreKitError> {
        let handle = ObserverHandle::shared();
        let orphans = handle.take_orphans();
        let queue_txs = tokio::task::spawn_blocking(Self::read_queue_transactions)
            .await
            .map_err(|e| StoreKitError::Failed(e.to_string()))?;
        let mut merged = Vec::with_capacity(orphans.len() + queue_txs.len());
        let mut seen = std::collections::HashSet::with_capacity(orphans.len() + queue_txs.len());
        for tx in orphans.into_iter().chain(queue_txs.into_iter()) {
            if seen.insert(tx.transaction_id.clone()) {
                merged.push(tx);
            }
        }
        Ok(merged)
    }

    async fn finish(&self, transaction_id: &str) -> Result<(), StoreKitError> {
        let id = transaction_id.to_owned();
        tokio::task::spawn_blocking(move || Self::finish_transaction(&id))
            .await
            .map_err(|e| StoreKitError::Failed(e.to_string()))?
    }

    async fn restore(&self) -> Result<Vec<StoreKitTransaction>, StoreKitError> {
        let handle = ObserverHandle::shared();
        let (tx, rx) = oneshot::channel();
        handle.register_restore_waiter(tx);

        tokio::task::spawn_blocking(Self::start_restore)
            .await
            .map_err(|e| StoreKitError::Failed(e.to_string()))?;

        rx.await.map_err(|_| StoreKitError::ChannelClosed)?
    }

    async fn products_available(
        &self,
        product_ids: &[&str],
    ) -> Result<ProductAvailabilityReport, StoreKitError> {
        let owned: Vec<String> = product_ids.iter().map(|s| (*s).to_owned()).collect();
        let rx = tokio::task::spawn_blocking(move || Self::start_products_request(owned))
            .await
            .map_err(|e| StoreKitError::Failed(e.to_string()))?;

        match tokio::time::timeout(PRODUCTS_REQUEST_TIMEOUT, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(StoreKitError::ChannelClosed),
            Err(_) => Err(StoreKitError::Failed(
                "products request timed out after 10 seconds".to_owned(),
            )),
        }
    }
}
