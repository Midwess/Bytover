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
use crate::file_api::file_extension::VecExtension;
use bincode::Options;
use core_services::logger;
pub use crux_core::bridge::Bridge;
pub use crux_core::{Core, Request};
use erased_serde::Serialize;
use js_sys::{Array, Reflect};
use n0_future::task::spawn;
use std::sync::LazyLock;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::Uint8Array;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, File, FileSystemWritableFileStream};
use serde::Deserialize;
use core_services::utils::never_send::NeverSend;
use crate::web_worker::core::{CoreRequest, CoreWorker};
use crate::web_worker::executor::{ExecutingWorker, NativeExecutorInput, NativeExecutorOutput, ShellRuntime, ThrottleShellRuntime};
use crate::web_worker::main::{WebWorkerBridge, WorkerMessage};

static WORKER: LazyLock<NeverSend<WebWorkerBridge<ExecutingWorker>>> = LazyLock::new(|| {
    let worker = NeverSend(WebWorkerBridge::spawn("native-executor"));
    worker.on_exhausted(|msg: WorkerMessage<NativeExecutorOutput>| {
        let msg = msg.message;
        spawn(async move {
            match msg {
                NativeExecutorOutput::ForwardCoreOperationOutput(request_id, data) => {
                    forward_core_operation_output(request_id, data).await;
                },
                NativeExecutorOutput::UpdateAppEvent(data) => {
                    update_app_event(data).await;
                }
                _ => {}
            }
        });
    });

    worker
});

static CORE_WORKER: LazyLock<NeverSend<WebWorkerBridge<CoreWorker>>> = LazyLock::new(|| NeverSend(WebWorkerBridge::spawn("core")));

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = core)]
    async fn forward_core_operation_output(request_id: u32, core_operation_output: Uint8Array);
    #[wasm_bindgen(js_namespace = core)]
    async fn update_app_event(app_event: Uint8Array);
}


#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn process_event(data: Uint8Array) -> Uint8Array {
    let msg = WorkerMessage::new(CoreRequest::Update(data));
    let Some(response) = CORE_WORKER.send(msg).await else {
        return Uint8Array::default()
    };

    response.message.0
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn handle_response(id: u32, data: Uint8Array) -> Uint8Array {
    let Some(response) = CORE_WORKER.send(WorkerMessage::new(CoreRequest::HandleResponse(id, data))).await else {
        return Uint8Array::default()
    };

    response.message.0
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn view() -> Uint8Array {
    let Some(response) = CORE_WORKER.send(WorkerMessage::new(CoreRequest::View)).await else {
        return Uint8Array::default()
    };

    response.message.0
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn init() {
    let host_info = config::get_host_info().unwrap();
    log::info!("Host info: {:?}", host_info);
    let _ = WORKER.send(WorkerMessage::new(NativeExecutorInput::Init(host_info))).await;
    log::info!("Initialized");
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn add_device_files(files: &Array) -> Uint8Array {
    let Some(response) = WORKER.send(WorkerMessage::new(NativeExecutorInput::AddDeviceFiles(files.clone()))).await else {
        return Uint8Array::default()
    };

    match response.message {
        NativeExecutorOutput::DeviceFiles(data) => data,
        _ => Uint8Array::default()
    }
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn get_device_file(resource_id: u64) -> Option<File> {
    let Some(response) = WORKER.send(WorkerMessage::new(NativeExecutorInput::GetDeviceFile(resource_id))).await else {
        return None
    };

    match response.message {
        NativeExecutorOutput::DeviceFile(file) => Some(file),
        _ => None
    }
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn load_thumbnail_bytes(resource_id: u64) -> Option<Uint8Array> {
    let Some(response) = WORKER.send(WorkerMessage::new(NativeExecutorInput::LoadThumbnailBytes(resource_id))).await else {
        return None
    };

    match response.message {
        NativeExecutorOutput::ThumbnailBytes(data) => Some(data),
        _ => None
    }
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn load_thumbnail_source(path: Uint8Array) -> Option<String> {
    let Some(response) = WORKER.send(WorkerMessage::new(NativeExecutorInput::LoadThumbnailSource(path))).await else {
        return None
    };

    match response.message {
        NativeExecutorOutput::ThumbnailSource(source) => source,
        _ => None
    }
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn download_file_from_cache(path: Uint8Array, writer: FileSystemWritableFileStream) {
    let _ = WORKER.send(WorkerMessage::new(NativeExecutorInput::DownloadFileFromCache { path, writer })).await;
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn execute(request_id: u32, effect: Uint8Array) -> Uint8Array {
    let Some(response) = WORKER.send(WorkerMessage::new(NativeExecutorInput::Execute(request_id, effect))).await else {
        return Uint8Array::default()
    };

    match response.message {
        NativeExecutorOutput::ExecuteResult(data) => data,
        _ => Uint8Array::default()
    }
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


pub fn deserialize<E: Serialize>(data: &Uint8Array) -> E where E: for<'de> Deserialize<'de> {
    let vec = data.to_vec();
    let options = bincode_options();
    let mut deser = bincode::Deserializer::from_slice(vec.as_slice(), options);
    let mut deserializer = <dyn erased_serde::Deserializer>::erase(&mut deser);
    let data: E = erased_serde::deserialize(&mut deserializer).expect("Failed to deserialize effect");
    data
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
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

    log::info!("Storage quota: {} bytes", quota_val.as_f64().unwrap_or(0.0));
    let quota = quota_val.as_f64().unwrap_or(0.0);
    if quota < 100.0 * 1024.0 * 1024.0 {
        log::info!("Storage quota less than 100MB ({} MB)", quota / 1024.0 / 1024.0);
        return false;
    }

    log::info!("Storage quota is OK");
    true
}
