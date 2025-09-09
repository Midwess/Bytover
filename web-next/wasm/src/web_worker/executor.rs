use gloo::worker::{HandlerId, Worker, WorkerScope};
use gloo_worker::Codec;
use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};
use core_services::logger::setup;
use shared::CoreOperation;
use crate::{deserialize, serialize};
use crate::web_worker::{CoreOperationEncoded, CoreOperationOutputEncoded};

#[derive(Serialize, Deserialize)]
pub struct ExecutingWorkerInput {
    #[serde(with = "serde_wasm_bindgen::preserve")]
    pub operation: CoreOperationEncoded
}

#[derive(Serialize, Deserialize)]
pub struct ExecutingWorkerOutput {
    #[serde(with = "serde_wasm_bindgen::preserve")]
    pub output: CoreOperationOutputEncoded
}

pub struct ExecutingWorker {
    processed_count: usize,
}

impl Worker for ExecutingWorker {
    type Message = ();
    type Input = ExecutingWorkerInput;
    type Output = String;

    fn create(scope: &WorkerScope<Self>) -> Self {
        setup();
        log::info!("Creating worker");

        Self {
            processed_count: 0,
        }
    }

    fn update(&mut self, scope: &WorkerScope<Self>, msg: Self::Message) {
        log::info!("Received message: {:?}", msg);
    }

    fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, id: HandlerId) {
        let operation: CoreOperation = deserialize(&msg.operation);
        log::info!("Received message: {:?}", operation);
    }
}
