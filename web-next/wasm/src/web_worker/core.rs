use crate::web_worker::bridge::{TrustedWorkerMessage, WorkerMessage};
use core_services::logger;
use core_services::wasm::extensions::VecExtension;
use crux_core::bridge::Bridge;
use crux_core::Core;
use devlog_sdk::distributed_id::init_scoped_id_generator;
use gloo_worker::Worker;
use js_sys::Uint8Array;
use shared::app::BitBridge;
use std::sync::LazyLock;

pub trait Handler<T> {
    type Output;
    fn handle(&self, input: T) -> Self::Output;
}

/// A web worker that dedicated to Core only
static CORE: LazyLock<Bridge<BitBridge>> = LazyLock::new(|| Bridge::new(Core::new()));

pub struct CoreWorker {}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum CoreWorkerOperation {
    Update(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    HandleResponse(u32, #[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    View
}

unsafe impl Send for CoreWorkerOperation {}
unsafe impl Sync for CoreWorkerOperation {}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CoreWorkerOperationOutput(#[serde(with = "serde_wasm_bindgen::preserve")] pub Uint8Array);

unsafe impl Send for CoreWorkerOperationOutput {}
unsafe impl Sync for CoreWorkerOperationOutput {}

impl Worker for CoreWorker {
    type Input = WorkerMessage<CoreWorkerOperation>;
    type Message = ();
    type Output = WorkerMessage<CoreWorkerOperationOutput>;

    fn create(_: &gloo_worker::WorkerScope<Self>) -> Self {
        logger::setup();
        init_scoped_id_generator("BitBridge".to_string());
        CoreWorker {}
    }

    fn update(&mut self, _: &gloo_worker::WorkerScope<Self>, _: Self::Message) {}

    fn received(&mut self, scope: &gloo_worker::WorkerScope<Self>, msg: Self::Input, id: gloo_worker::HandlerId) {
        let msg_id = msg.id().to_string();
        match msg.message {
            CoreWorkerOperation::Update(event_data) => {
                let data = CORE.process_event(event_data.to_vec().as_slice()).unwrap_or_default();
                scope.respond(
                    id,
                    WorkerMessage::response(msg_id, CoreWorkerOperationOutput(data.into_uint_array()))
                );
            }
            CoreWorkerOperation::HandleResponse(request_id, response_data) => {
                let data = CORE.handle_response(request_id, response_data.to_vec().as_slice()).unwrap_or_default();
                scope.respond(
                    id,
                    WorkerMessage::response(msg_id, CoreWorkerOperationOutput(data.into_uint_array()))
                );
            }
            CoreWorkerOperation::View => {
                let data = CORE.view().unwrap_or_default();
                scope.respond(
                    id,
                    WorkerMessage::response(msg_id, CoreWorkerOperationOutput(data.into_uint_array()))
                );
            }
        }
    }
}
