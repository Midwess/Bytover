#![allow(deprecated)]

use async_trait::async_trait;
use objc2::rc::Retained;
use objc2_store_kit::{SKPaymentQueue, SKPaymentTransaction, SKPaymentTransactionState};

use super::{StoreKitClient, StoreKitError, StoreKitTransaction};

const DELEGATE_NOT_WIRED_MSG: &str =
    "StoreKit purchase flow requires the native SKPaymentTransactionObserver delegate to be wired; \
     this build cannot complete purchases until entitlement provisioning and the ObjC delegate class are finalized in follow-up work";

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

    fn read_transactions() -> Vec<StoreKitTransaction> {
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
}

fn map_transaction(tx: &SKPaymentTransaction) -> Option<StoreKitTransaction> {
    let state = unsafe { tx.transactionState() };
    let is_done = state.0 == SKPaymentTransactionState::Purchased.0 || state.0 == SKPaymentTransactionState::Restored.0;
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
                "canMakePayments returned false — in-app purchases are disabled on this device".to_owned(),
            ));
        }
        log::warn!(
            "[storekit] purchase({product_id}) requested but observer delegate is not yet wired — {DELEGATE_NOT_WIRED_MSG}"
        );
        Err(StoreKitError::Failed(DELEGATE_NOT_WIRED_MSG.to_owned()))
    }

    async fn unfinished_transactions(&self) -> Result<Vec<StoreKitTransaction>, StoreKitError> {
        let txs = tokio::task::spawn_blocking(Self::read_transactions)
            .await
            .map_err(|e| StoreKitError::Failed(e.to_string()))?;
        Ok(txs)
    }

    async fn finish(&self, transaction_id: &str) -> Result<(), StoreKitError> {
        let id = transaction_id.to_owned();
        tokio::task::spawn_blocking(move || Self::finish_transaction(&id))
            .await
            .map_err(|e| StoreKitError::Failed(e.to_string()))?
    }

    async fn restore(&self) -> Result<Vec<StoreKitTransaction>, StoreKitError> {
        log::warn!("[storekit] restore requested but observer delegate is not yet wired — {DELEGATE_NOT_WIRED_MSG}");
        Err(StoreKitError::Failed(DELEGATE_NOT_WIRED_MSG.to_owned()))
    }
}

