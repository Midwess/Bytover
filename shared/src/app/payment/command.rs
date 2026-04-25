use crate::app::core::command::AppCommand;
use crate::app::core::extensions::CoreCommandContextUtils;
use crate::app::operations::dialog::{AlertDialog, DialogOperation};
use crate::app::operations::rpc::RpcOperation;
use crate::app::operations::storekit::{StoreKitOperation, StoreKitOperationOutput, StoreKitTransactionDto};
use crate::app::payment::module::{InFlight, PaymentEvent, ProductId};
use crate::app::AppEvent;
use crate::errors::CoreError;
use crate::protocol::rpc::cloud_server::SubmitStoreKitResult;

impl AppCommand {
    pub async fn purchase(self, product_id: ProductId) -> Result<(), CoreError> {
        log::info!("[payment] purchase begin: product_id={product_id}");
        self.notify_event(PaymentEvent::BeginLoading(InFlight::Purchase(product_id.clone())));

        let outcome = match self.run(StoreKitOperation::purchase(product_id.clone())).await {
            StoreKitOperationOutput::Transaction(dto) => self.submit_and_finish(dto).await,
            StoreKitOperationOutput::Failed(message) => {
                log::warn!("[payment] StoreKit purchase failed: {message}");
                self.surface_failure(&message).await;
                Ok(())
            }
            other => {
                log::warn!("[payment] StoreKit purchase returned unexpected output: {other:?}");
                self.surface_failure("StoreKit returned an unexpected response").await;
                Ok(())
            }
        };

        self.notify_event(PaymentEvent::EndLoading);
        outcome
    }

    pub async fn restore(self) -> Result<(), CoreError> {
        log::info!("[payment] restore begin");
        self.notify_event(PaymentEvent::BeginLoading(InFlight::Restore));

        let outcome = match self.run(StoreKitOperation::restore_all()).await {
            StoreKitOperationOutput::Transactions(list) => {
                self.submit_serially(list, "restore").await;
                Ok(())
            }
            StoreKitOperationOutput::Failed(message) => {
                log::warn!("[payment] StoreKit restore failed: {message}");
                self.surface_failure(&message).await;
                Ok(())
            }
            other => {
                log::warn!("[payment] StoreKit restore returned unexpected output: {other:?}");
                self.surface_failure("StoreKit returned an unexpected response").await;
                Ok(())
            }
        };

        self.notify_event(PaymentEvent::EndLoading);
        outcome
    }

    pub async fn resume_pending(self) -> Result<(), CoreError> {
        log::info!("[payment] resume_pending begin");
        self.notify_event(PaymentEvent::BeginLoading(InFlight::ResumePending));

        let outcome = match self.run(StoreKitOperation::fetch_unfinished()).await {
            StoreKitOperationOutput::Transactions(list) => {
                if list.is_empty() {
                    log::info!("[payment] resume_pending: nothing to resume");
                    Ok(())
                } else {
                    self.submit_serially(list, "resume_pending").await;
                    Ok(())
                }
            }
            StoreKitOperationOutput::Failed(message) => {
                log::warn!("[payment] StoreKit fetch_unfinished failed: {message}");
                Ok(())
            }
            other => {
                log::warn!("[payment] StoreKit fetch_unfinished returned unexpected output: {other:?}");
                Ok(())
            }
        };

        self.notify_event(PaymentEvent::EndLoading);
        outcome
    }

    async fn submit_and_finish(&self, dto: StoreKitTransactionDto) -> Result<(), CoreError> {
        log::info!(
            "[payment] submit_and_finish: transaction_id={} product_id={}",
            dto.transaction_id, dto.product_id
        );
        let transaction_id = dto.transaction_id.clone();
        let result = match self
            .run(RpcOperation::submit_storekit_transaction(dto.transaction_id, dto.product_id))
            .await
        {
            Ok(r) => r,
            Err(error) => {
                let message = error.to_string();
                log::warn!("[payment] submit_storekit_transaction transport failed: {message}");
                self.surface_failure(&message).await;
                return Err(error);
            }
        };

        match result {
            SubmitStoreKitResult::Completed { capabilities, .. } => {
                log::info!("[payment] submission Completed for transaction_id={transaction_id}");
                self.notify_event(PaymentEvent::CapabilitiesLoaded(capabilities));
                let _ = self.run(StoreKitOperation::finish_transaction(transaction_id)).await;
                self.notify_event(PaymentEvent::SubmissionCompleted);
            }
            SubmitStoreKitResult::Rejected { code, message } => {
                let display = message.clone().unwrap_or_else(|| format!("Payment rejected ({code:?})"));
                log::warn!("[payment] submission Rejected: code={code:?} message={message:?}");
                self.surface_failure(&display).await;
            }
        }

        Ok(())
    }

    async fn submit_serially(&self, transactions: Vec<StoreKitTransactionDto>, label: &'static str) {
        let total = transactions.len();
        let mut succeeded = 0usize;

        for (index, dto) in transactions.into_iter().enumerate() {
            log::info!("[payment] {label}: submitting [{}/{total}] transaction_id={}", index + 1, dto.transaction_id);
            match self.submit_and_finish(dto).await {
                Ok(()) => succeeded += 1,
                Err(error) => {
                    log::warn!(
                        "[payment] {label}: aborting at [{}/{total}]: succeeded={succeeded}/{total} err={error}",
                        index + 1
                    );
                    break;
                }
            }
        }
    }

    async fn surface_failure(&self, message: &str) {
        self.run(DialogOperation::alert(AlertDialog::alert(message.to_owned()))).await;
        self.notify_event(PaymentEvent::SubmissionFailed(message.to_owned()));
    }

    pub async fn refresh_payment_capabilities(&self) {
        match RpcOperation::get_capabilities().into_future(self.ctx()).await {
            Ok(caps) => {
                self.notify_event(AppEvent::Payment(PaymentEvent::CapabilitiesLoaded(caps)));
            }
            Err(err) => {
                self.notify_event(AppEvent::Payment(PaymentEvent::RefreshCapabilitiesFailed(format!("{err:?}"))));
            }
        }
    }

    pub async fn report_p2p_bytes_used(&self, delta: u64) {
        match RpcOperation::report_p2p_bytes_used(delta).into_future(self.ctx()).await {
            Ok(caps) => {
                self.notify_event(AppEvent::Payment(PaymentEvent::CapabilitiesLoaded(caps)));
            }
            Err(err) => {
                log::warn!("[payment] report_p2p_bytes_used failed: delta={delta} err={err:?}");
            }
        }
    }
}
