use anyhow::anyhow;
use wasm_bindgen::JsValue;
use shared::core_transfer_protocol::public_cloud::cloud_service::CloudTransferErrors;

#[derive(Debug)]
pub struct JsError(pub JsValue);

impl From<JsError> for CloudTransferErrors {
    fn from(value: JsError) -> Self {
        CloudTransferErrors::InternalError(anyhow!("{:?}", value))
    }
}
