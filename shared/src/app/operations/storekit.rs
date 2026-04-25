use std::future::Future;

use crux_core::capability::Operation;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::app::AppRequestBuilder;

use super::{CoreOperation, CoreOperationOutput};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoreKitTransactionDto {
    pub transaction_id: String,
    pub product_id: String,
    pub original_transaction_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StoreKitOperation {
    Purchase { product_id: String },
    RestoreAll,
    FetchUnfinished,
    FinishTransaction { transaction_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StoreKitOperationOutput {
    Transaction(StoreKitTransactionDto),
    Transactions(Vec<StoreKitTransactionDto>),
    Finished,
    Failed(String),
}

impl Operation for StoreKitOperation {
    type Output = StoreKitOperationOutput;
}

fn extract(output: CoreOperationOutput, label: &'static str) -> StoreKitOperationOutput {
    match output {
        CoreOperationOutput::StoreKit(out) => out,
        CoreOperationOutput::Error(error) => StoreKitOperationOutput::Failed(error.to_string()),
        other => StoreKitOperationOutput::Failed(format!("invalid output for {label}: {other:?}")),
    }
}

impl StoreKitOperation {
    pub fn purchase(product_id: String) -> AppRequestBuilder<impl Future<Output = StoreKitOperationOutput>> {
        Command::request_from_shell(CoreOperation::StoreKit(StoreKitOperation::Purchase { product_id }))
            .map(|res| extract(res, "StoreKitOperation::Purchase"))
    }

    pub fn restore_all() -> AppRequestBuilder<impl Future<Output = StoreKitOperationOutput>> {
        Command::request_from_shell(CoreOperation::StoreKit(StoreKitOperation::RestoreAll))
            .map(|res| extract(res, "StoreKitOperation::RestoreAll"))
    }

    pub fn fetch_unfinished() -> AppRequestBuilder<impl Future<Output = StoreKitOperationOutput>> {
        Command::request_from_shell(CoreOperation::StoreKit(StoreKitOperation::FetchUnfinished))
            .map(|res| extract(res, "StoreKitOperation::FetchUnfinished"))
    }

    pub fn finish_transaction(transaction_id: String) -> AppRequestBuilder<impl Future<Output = StoreKitOperationOutput>> {
        Command::request_from_shell(CoreOperation::StoreKit(StoreKitOperation::FinishTransaction { transaction_id }))
            .map(|res| extract(res, "StoreKitOperation::FinishTransaction"))
    }
}
