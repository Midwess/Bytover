use anyhow::anyhow;
use shared::protocol::public_cloud::cloud_service::CloudTransferErrors;
use wasm_bindgen::JsValue;

#[derive(Debug)]
pub struct JsError(pub JsValue);

impl From<JsError> for CloudTransferErrors {
    fn from(value: JsError) -> Self {
        CloudTransferErrors::InternalError(anyhow!("{:?}", value))
    }
}
