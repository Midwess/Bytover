use std::ops::Deref;
use gloo::worker::{HandlerId, Worker, WorkerScope};
use gloo_worker::Codec;
use js_sys::Atomics::store;
use js_sys::{Array, Uint8Array};
use n0_future::task::spawn;
use serde::{Deserialize, Serialize};
use shared::CoreOperation;
use crate::{deserialize, serialize};
use crate::di_container::DiContainer;
use crate::executor::executor::NativeExecutor;
use crate::file_api::storage::FileStorage;
use crate::web_worker::{CoreOperationEncoded, CoreOperationOutputEncoded};
use crate::web_worker::main::WorkerMessage;

#[derive(Serialize, Deserialize)]
pub enum NativeExecutorOperation {
    HandleEffect(u32, #[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    Init,
    AddDeviceFiles(#[serde(with = "serde_wasm_bindgen::preserve")] Array),
    GetDeviceFile(u64),
    LoadThumbnailBytes(u64),
    DownloadFile {
        #[serde(with = "serde_wasm_bindgen::preserve")]
        path: Uint8Array,
        #[serde(with = "serde_wasm_bindgen::preserve")]
        writer: web_sys::FileSystemWritableFileStream
    },
}

unsafe impl Send for NativeExecutorOperation {}
unsafe impl Sync for NativeExecutorOperation {}

#[derive(Serialize, Deserialize)]
pub enum NativeExecutorOutput {
    Effects(u32, #[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    Void,
    ThumbnailBytes(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
}

unsafe impl Send for NativeExecutorOutput {}
unsafe impl Sync for NativeExecutorOutput {}

pub struct ExecutingWorker {
    storage: FileStorage,
    native_executor: &'static NativeExecutor,
}

impl Worker for ExecutingWorker {
    type Message = ();
    type Input = WorkerMessage<NativeExecutorOperation>;
    type Output = WorkerMessage<NativeExecutorOutput>;

    fn create(scope: &WorkerScope<Self>) -> Self {
        log::info!("Creating worker");

        let di_instance = DiContainer::get_instance();

        Self {
            storage: di_instance.file_storage(),
            native_executor: di_instance.get_native_executor()
        }
    }

    fn update(&mut self, scope: &WorkerScope<Self>, msg: Self::Message) {
        log::info!("Received message: {:?}", msg);
    }

    fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, id: HandlerId) {
        let scope = scope.clone();
        let native_executor = self.native_executor;
        spawn(async move {
            match msg.deref() {
                NativeExecutorOperation::HandleEffect(request_id, data) => {
                    let effect: CoreOperation = deserialize(&data);
                    let output = native_executor.handle(*request_id, effect).await;
                    scope.respond(id, WorkerMessage::new(NativeExecutorOutput::Effects(*request_id, serialize(&output))))
                }
            }
        });
    }
}
