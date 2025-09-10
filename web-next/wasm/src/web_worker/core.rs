use gloo_worker::Worker;
use std::sync::LazyLock;
use crux_core::bridge::Bridge;
use crux_core::{App, Core};
use js_sys::Uint8Array;
use shared::app::BitBridge;
use crate::file_api::file_extension::VecExtension;
use crate::web_worker::main::{TrustedWorkerMessage, WorkerMessage};

/// A web worker that dedicated to Core only
static CORE: LazyLock<Bridge<BitBridge>> = LazyLock::new(|| Bridge::new(Core::new()));

pub struct CoreWorker {}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum CoreRequest {
    Update(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    HandleResponse(u32, #[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    View,
}

unsafe impl Send for CoreRequest {}
unsafe impl Sync for CoreRequest {}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CoreResponse(#[serde(with = "serde_wasm_bindgen::preserve")] pub Uint8Array);

unsafe impl Send for CoreResponse {}
unsafe impl Sync for CoreResponse {}

impl Worker for CoreWorker {
    type Message = ();
    type Input = WorkerMessage<CoreRequest>;
    type Output = WorkerMessage<CoreResponse>;

    fn create(_: &gloo_worker::WorkerScope<Self>) -> Self {
        log::info!("Core worker created");
        CoreWorker {}
    }

    fn update(&mut self, _: &gloo_worker::WorkerScope<Self>, _: Self::Message) {}

    fn received(&mut self, scope: &gloo_worker::WorkerScope<Self>, msg: Self::Input, id: gloo_worker::HandlerId) {
        let msg_id = msg.id().to_string();
        match msg.message {
            CoreRequest::Update(event_data) => {
                let data = CORE.process_event(event_data.to_vec().as_slice()).unwrap_or_default();
                scope.respond(id, WorkerMessage::response(msg_id, CoreResponse(data.into_uint_array())));
            }
            CoreRequest::HandleResponse(request_id, response_data) => {
                let data = CORE.handle_response(request_id, response_data.to_vec().as_slice()).unwrap_or_default();
                scope.respond(id, WorkerMessage::response(msg_id, CoreResponse(data.into_uint_array())));
            }
            CoreRequest::View => {
                let data = CORE.view().unwrap_or_default();
                scope.respond(id, WorkerMessage::response(msg_id, CoreResponse(data.into_uint_array())));
            }
        }
    }
}
