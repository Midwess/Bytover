use tauri::AppHandle;

use shared::app::payment::{PaymentEvent, PREMIUM_PRODUCT_ID};

use crate::process_event;
use crate::storekit::{self, ProductAvailabilityReport};

#[tauri::command]
pub async fn purchase_premium(app_handle: AppHandle) {
    log::info!("[payment] purchase_premium command invoked for {PREMIUM_PRODUCT_ID}");
    process_event(PaymentEvent::Purchase(PREMIUM_PRODUCT_ID.to_owned()), app_handle).await;
}

#[tauri::command]
pub async fn restore_purchases(app_handle: AppHandle) {
    log::info!("[payment] restore_purchases command invoked");
    process_event(PaymentEvent::Restore, app_handle).await;
}

#[tauri::command]
pub async fn resume_pending_transactions(app_handle: AppHandle) {
    log::info!("[payment] resume_pending_transactions command invoked");
    process_event(PaymentEvent::ResumePending, app_handle).await;
}

#[tauri::command]
pub async fn check_storekit_product_availability() -> Result<ProductAvailabilityReport, String> {
    let client = storekit::default_client();
    client
        .products_available(&[PREMIUM_PRODUCT_ID])
        .await
        .map_err(|e| e.to_string())
}
