use serde::Serialize;
use shared::entities::capabilities::Plan;
use shared::protocol::rpc::payment::PayResult;
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
    log::info!("[storekit] purchase_premium command invoked for {PREMIUM_PRODUCT_ID}");
    let client = storekit::default_client();
    let transaction = client
        .purchase(PREMIUM_PRODUCT_ID)
        .await
        .map_err(|e| {
            log::error!("[storekit] purchase_premium client.purchase failed: {e}");
            e.to_string()
        })?;

    log::info!(
        "[storekit] purchase_premium got transaction {} for {}, submitting to backend",
        transaction.transaction_id, transaction.product_id,
    );
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
    let di = native::di_container::DiContainer::get_instance();
    let app_server = di.get_authentication_server();

    let pay_result = app_server
        .pay_storekit(tx.transaction_id.clone(), tx.product_id.clone())
        .await
        .map_err(|e| {
            log::error!("[payment] pay_storekit failed: {e}");
            e.to_string()
        })?;

    match pay_result {
        PayResult::Completed {
            payment_statement_id,
            product_id,
            transaction_id,
        } => {
            client.finish(&transaction_id).await.map_err(|e| e.to_string())?;

            let caps = di
                .get_cloud_server()
                .get_capabilities()
                .await
                .map_err(|e| {
                    log::warn!("[payment] capabilities refresh after pay failed: {e}");
                    e.to_string()
                })?;

            let _ = app_handle.emit("capabilities-changed", ());

            Ok(PurchaseOutcome {
                upgraded: caps.plan == Plan::Paid,
                duplicate: false,
                product_id,
                payment_statement_id: Some(payment_statement_id),
            })
        }
        PayResult::Rejected { message } => Err(format!("payment rejected: {message}")),
    }
}
