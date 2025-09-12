extern crate core;

pub mod config;
pub mod core_api_impl;
pub mod di_container;
mod errors;
pub mod executor;
pub mod file_api;
pub mod network;
pub mod repository;
pub mod web_worker;

// /shared/src/lib.rs
use crate::di_container::DiContainer;
use crate::executor::executor::NativeExecutor;
use crate::file_api::file_extension::VecExtension;
use crate::file_api::storage::FileStorage;
use bincode::Options;
use core_services::logger;
pub use crux_core::bridge::Bridge;
pub use crux_core::{Core, Request};
use erased_serde::Serialize;
use file_api::path_extension::WebExtLocalResourcePath;
use futures::lock::Mutex;
use js_sys::{Array, Reflect};
use n0_future::task::{spawn, JoinHandle};
use n0_future::time;
use n0_future::time::Interval;
use shared::app::file_system::file::LocalResourcePath;
use shared::app::operations::CoreOperationOutput;
use shared::app::AppEvent;
use shared::CoreOperation;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::Uint8Array;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, File, FileSystemWritableFileStream};
use serde::Deserialize;
use core_services::utils::never_send::NeverSend;
use crate::web_worker::core::{CoreWorkerOperation, CoreWorker};
use crate::web_worker::bridge::{WebWorkerBridge, WorkerMessage};

static CORE_WORKER: LazyLock<NeverSend<WebWorkerBridge<CoreWorker>>> = LazyLock::new(|| NeverSend(WebWorkerBridge::spawn("core-worker")));

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = core)]
    async fn forward_core_operation_output(request_id: u32, core_operation_output: Uint8Array);
    #[wasm_bindgen(js_namespace = core)]
    async fn update_app_event(app_event: Uint8Array);
}

pub struct ShellRuntime {}

impl ShellRuntime {
    fn forward_core_operation_output(self: Arc<Self>, request_id: u32, output: CoreOperationOutput) -> JoinHandle<()> {
        spawn(async move {
            let serialized_output = serialize(&output);
            forward_core_operation_output(request_id, serialized_output).await;
        })
    }

    fn update(self: Arc<Self>, event: AppEvent) -> JoinHandle<()> {
        spawn(async move {
            let serialized_event = serialize(&event);
            update_app_event(serialized_event).await;
        })
    }
}

pub struct ThrottleShellRuntime {
    latest_event: Arc<Mutex<Option<(u32, CoreOperationOutput)>>>
}

impl ThrottleShellRuntime {
    pub fn new(shell_runtime: Arc<ShellRuntime>, delay: Duration) -> Self {
        let latest_event = Arc::new(Mutex::new(None::<(u32, CoreOperationOutput)>));
        let latest_event_clone = latest_event.clone();
        let shell_runtime_clone = shell_runtime.clone();

        spawn(async move {
            let mut interval: Interval = time::interval(delay);
            interval.tick().await;

            loop {
                interval.tick().await;

                let event_to_send = {
                    let mut latest = latest_event_clone.lock().await;
                    latest.take()
                };

                if let Some(event) = event_to_send {
                    let _ = shell_runtime_clone.clone().forward_core_operation_output(event.0, event.1).await;
                }
            }
        });

        Self { latest_event }
    }

    pub async fn send(&self, request_id: u32, event: CoreOperationOutput) {
        let mut latest = self.latest_event.lock().await;
        *latest = Some((request_id, event));
    }
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn process_event(data: Uint8Array) -> Uint8Array {
    let msg = WorkerMessage::new(CoreWorkerOperation::Update(data));
    let Some(response) = CORE_WORKER.send(msg).await else {
        return Uint8Array::default()
    };

    response.message.0
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn handle_response(id: u32, data: Uint8Array) -> Uint8Array {
    let Some(response) = CORE_WORKER.send(WorkerMessage::new(CoreWorkerOperation::HandleResponse(id, data))).await else {
        return Uint8Array::default()
    };

    response.message.0
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn view() -> Uint8Array {
    let Some(response) = CORE_WORKER.send(WorkerMessage::new(CoreWorkerOperation::View)).await else {
        return Uint8Array::default()
    };

    response.message.0
}

pub fn serialize<E: Serialize>(data: &E) -> Uint8Array {
    let options = bincode_options();
    let mut buffer = Vec::new();
    let mut serializer = bincode::Serializer::new(&mut buffer, options);
    erased_serde::serialize(data, &mut serializer).unwrap();
    buffer.into_uint_array()
}

fn bincode_options() -> impl Options + Copy {
    bincode::DefaultOptions::new().with_fixint_encoding().allow_trailing_bytes()
}

#[wasm_bindgen]
pub struct NativeProcessor {
    executor: &'static NativeExecutor,
    storage: FileStorage
}

#[wasm_bindgen]
impl NativeProcessor {
    pub async fn is_compatible() -> bool {
        logger::setup();
        let Some(with_browser) = window() else {
            log::info!("No window");
            return false
        };

        if with_browser.is_null() || with_browser.is_undefined() {
            log::info!("Window is null");
            return false
        }

        let Ok(with_cache) = with_browser.caches() else {
            log::info!("No caches");
            return false
        };

        if with_cache.is_null() || with_cache.is_undefined() {
            log::info!("Caches is null");
            return false
        }

        let storage = with_browser.navigator().storage();

        if storage.is_null() || storage.is_undefined() {
            log::info!("Storage is null");
            return false
        }

        let Ok(estimate_fut) = storage.estimate() else {
            return false;
        };

        let Ok(quota) = JsFuture::from(estimate_fut).await else {
            log::info!("Cannot estimate storage quota");
            return false
        };

        if quota.is_null() || quota.is_undefined() {
            log::info!("Quota is null");
            return false
        }

        let quota_val = Reflect::get(&quota, &JsValue::from_str("quota")).unwrap_or(JsValue::UNDEFINED);

        if quota_val.is_null() || quota_val.is_undefined() {
            log::info!("quota field missing");
            return false;
        }

        let quota = quota_val.as_f64().unwrap_or(0.0);
        if quota < 100.0 * 1024.0 * 1024.0 {
            log::info!("Storage quota less than 100MB ({} MB)", quota / 1024.0 / 1024.0);
            return false;
        }

        true
    }

    pub async fn init() -> Self {
        let di_container = DiContainer::get_instance();
        di_container.init(Arc::new(ShellRuntime {})).await;

        Self {
            storage: di_container.file_storage(),
            executor: di_container.get_native_executor().await
        }
    }

    pub async fn add_device_files(&self, files: &Array) -> Uint8Array {
        let paths = self.storage.add(files).await;

        serialize(&paths)
    }

    pub async fn get_device_file(&self, resource_id: u64) -> Option<File> {
        let file = self.storage.get(resource_id).await;
        file.map(|it| it.file.0)
    }

    pub async fn load_thumbnail_bytes(&self, resource_id: u64) -> Option<Uint8Array> {
        let repository = DiContainer::get_instance().get_local_resource_repository().await;
        let path = LocalResourcePath::cache("thumbnails", resource_id.to_string());
        let Ok(mut reader) = repository.read(path, 1024 * 256).await else {
            return None
        };

        let Ok(data) = reader.read_all().await else { return None };

        Some(data.into_uint_array())
    }

    pub async fn load_thumbnail_source(&self, path: Vec<u8>) -> Option<String> {
        let options = bincode_options();
        let mut deser = bincode::Deserializer::from_slice(&path, options);
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(&mut deser);
        let path: LocalResourcePath = erased_serde::deserialize(&mut deserializer).expect("Failed to deserialize effect");

        if let LocalResourcePath::AbsolutePath(path) = path {
            return Some(path)
        }

        let Some(resource_id) = path.thumbnail_resource_id() else {
            return None
        };

        let Some(data) = self.load_thumbnail_bytes(resource_id).await else {
            return None
        };

        let blob_options = web_sys::BlobPropertyBag::new();
        blob_options.set_type("image/png");

        let parts = Array::new();
        parts.push(&data);

        let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &blob_options) else {
            return None
        };

        let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
            return None
        };

        Some(url)
    }

    pub async fn download_file_from_cache(&self, path: Vec<u8>, writer: FileSystemWritableFileStream) {
        let options = bincode_options();
        let mut deser = bincode::Deserializer::from_slice(&path, options);
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(&mut deser);
        let path: LocalResourcePath = erased_serde::deserialize(&mut deserializer).expect("Failed to deserialize effect");
        let repository = DiContainer::get_instance().get_local_resource_repository().await;
        let Ok(mut reader) = repository.read(path, 1024 * 256).await else {
            let _ = JsFuture::from(writer.close()).await;
            return;
        };

        while let Some(data) = reader.next().await.unwrap() {
            let Ok(fut) = writer.write_with_u8_array(&data) else {
                break;
            };

            if let Err(e) = JsFuture::from(fut).await {
                log::error!("Failed to write to file: {:?}", e);
                break;
            }
        }

        let _ = JsFuture::from(writer.close()).await;
    }

    pub async fn execute(&self, request_id: u32, effect: Uint8Array) -> Uint8Array {
        let effect: CoreOperation = deserialize(&effect);
        let output = self.executor.handle(request_id, effect).await;
        handle_response(request_id, serialize(&output)).await
    }
}

pub fn deserialize<E: Serialize>(data: &Uint8Array) -> E where E: for<'de> Deserialize<'de> {
    let vec = data.to_vec();
    let options = bincode_options();
    let mut deser = bincode::Deserializer::from_slice(vec.as_slice(), options);
    let mut deserializer = <dyn erased_serde::Deserializer>::erase(&mut deser);
    let data: E = erased_serde::deserialize(&mut deserializer).expect("Failed to deserialize effect");
    data
}
