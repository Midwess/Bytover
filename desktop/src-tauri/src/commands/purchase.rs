use serde::Serialize;
use shared::protocol::rpc::cloud_server::{SubmitStoreKitRejectionCode, SubmitStoreKitResult};
use tauri::{AppHandle, Emitter};

use crate::storekit::{self, ProductAvailabilityReport, PREMIUM_PRODUCT_ID};

#[derive(Debug, Serialize)]
pub struct PurchaseOutcome {
    pub upgraded: bool,
    pub duplicate: bool,
    pub product_id: String,
    pub payment_statement_id: Option<u64>,
}

#[tauri::command]
pub async fn purchase_premium(app_handle: AppHandle) -> Result<PurchaseOutcome, String> {
    let client = storekit::default_client();
    let transaction = client
        .purchase(PREMIUM_PRODUCT_ID)
        .await
        .map_err(|e| e.to_string())?;

    submit_and_finish(app_handle, client.as_ref(), &transaction).await
}

#[tauri::command]
pub async fn restore_purchases(app_handle: AppHandle) -> Result<Vec<PurchaseOutcome>, String> {
    let client = storekit::default_client();
    let transactions = client.restore().await.map_err(|e| e.to_string())?;

    let mut outcomes = Vec::with_capacity(transactions.len());
    for tx in transactions {
        let outcome = submit_and_finish(app_handle.clone(), client.as_ref(), &tx).await?;
        outcomes.push(outcome);
    }
    Ok(outcomes)
}

#[tauri::command]
pub async fn check_storekit_product_availability() -> Result<ProductAvailabilityReport, String> {
    let client = storekit::default_client();
    client
        .products_available(&[PREMIUM_PRODUCT_ID])
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resume_pending_transactions(app_handle: AppHandle) -> Result<usize, String> {
    let client = storekit::default_client();
    let pending = client
        .unfinished_transactions()
        .await
        .map_err(|e| e.to_string())?;

    let mut resumed = 0usize;
    for tx in pending {
        match submit_and_finish(app_handle.clone(), client.as_ref(), &tx).await {
            Ok(_) => resumed += 1,
            Err(err) => {
                log::warn!("[storekit] failed to resume transaction {}: {err}", tx.transaction_id);
            }
        }
    }
    Ok(resumed)
}

async fn submit_and_finish(
    app_handle: AppHandle,
    client: &dyn storekit::StoreKitClient,
    tx: &storekit::StoreKitTransaction,
) -> Result<PurchaseOutcome, String> {
    let cloud_server = native::di_container::DiContainer::get_instance().get_cloud_server();
    let result = cloud_server
        .submit_storekit_transaction(tx.transaction_id.clone(), tx.product_id.clone())
        .await
        .map_err(|e| e.to_string())?;

    match result {
        SubmitStoreKitResult::Success {
            payment_statement_id,
            product_id,
            duplicate,
            upgraded_to_paid,
            capabilities: _,
        } => {
            client.finish(&tx.transaction_id).await.map_err(|e| e.to_string())?;
            let _ = app_handle.emit("capabilities-changed", ());
            Ok(PurchaseOutcome {
                upgraded: upgraded_to_paid,
                duplicate,
                product_id,
                payment_statement_id: Some(payment_statement_id),
            })
        }
        SubmitStoreKitResult::Rejected { code, message } => {
            let code_label = match code {
                SubmitStoreKitRejectionCode::Unknown => "unknown",
                SubmitStoreKitRejectionCode::NotFound => "not_found",
                SubmitStoreKitRejectionCode::BundleMismatch => "bundle_mismatch",
                SubmitStoreKitRejectionCode::InvalidSignature => "invalid_signature",
                SubmitStoreKitRejectionCode::EnvMismatch => "env_mismatch",
                SubmitStoreKitRejectionCode::AppleApiError => "apple_api_error",
                SubmitStoreKitRejectionCode::ProductUnknown => "product_unknown",
                SubmitStoreKitRejectionCode::ConfigMissing => "config_missing",
                SubmitStoreKitRejectionCode::Internal => "internal",
            };
            Err(format!(
                "storekit rejected ({code_label}): {}",
                message.unwrap_or_else(|| "no message".to_owned())
            ))
        }
    }
}
