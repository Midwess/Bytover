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
use crate::file_api::device_file::FileStorage;
use crate::file_api::file_extension::VecExtension;
use crate::file_api::opfs::OPFS_WORKER;
use crate::file_api::path_extension::WebExtLocalResourcePath;
use crate::web_worker::bridge::{WebWorkerBridge, WorkerMessage};
use crate::web_worker::core::{CoreWorker, CoreWorkerOperation};
use crate::web_worker::opfs::{FileOperation, OpfsOperation, OpfsOperationOutput};
use bincode::Options;
use core_services::logger;
use core_services::utils::never_send::NeverSend;
pub use crux_core::bridge::Bridge;
pub use crux_core::{Core, Request};
use erased_serde::Serialize;
use js_sys::{Array, Promise};
use serde::Deserialize;
use shared::entities::file_system::file::LocalResourcePath;
use shared::CoreOperation;
use std::sync::LazyLock;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::Uint8Array;
use web_sys::{window, File};

static CORE_WORKER: LazyLock<NeverSend<WebWorkerBridge<CoreWorker>>> =
    LazyLock::new(|| NeverSend(WebWorkerBridge::spawn("core-worker")));

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = core)]
    async fn forward_core_operation_output(request_id: u32, core_operation_output: Uint8Array);
    #[wasm_bindgen(js_namespace = core)]
    async fn update_app_event(app_event: Uint8Array);

    /// OPFS
    #[wasm_bindgen(js_namespace = ["navigator", "storage"], js_name = getDirectory)]
    fn get_directory() -> Promise;
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

#[wasm_bindgen]
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

    true
}

#[wasm_bindgen]
pub async fn init() {
    let di_container = DiContainer::get_instance();
    di_container.init().await;
}

#[wasm_bindgen]
pub async fn add_device_files(files: &Array) -> Uint8Array {
    let storage = DiContainer::get_instance().file_storage();
    let paths = storage.add(files).await;

    serialize(&paths)
}

#[wasm_bindgen]
pub async fn get_device_file(resource_id: u64) -> Option<File> {
    let storage = DiContainer::get_instance().file_storage();
    let file = storage.get(resource_id).await;
    file.map(|it| it.file.0)
}

#[wasm_bindgen]
pub async fn get_download_url(path: Uint8Array) -> Option<String> {
    let path: LocalResourcePath = deserialize(&path);
    let opfs_path = match path {
        LocalResourcePath::AbsolutePath(path) => return Some(path),
        LocalResourcePath::PlatformIdentifier(_) => path.opfs_path()?,
        _ => return None
    };

    let _ = OPFS_WORKER
        .send(WorkerMessage::new(OpfsOperation {
            file_path: opfs_path.clone(),
            operation: FileOperation::Open
        }))
        .await;

    let response = OPFS_WORKER
        .send(WorkerMessage::new(OpfsOperation {
            file_path: opfs_path,
            operation: FileOperation::GenerateSource
        }))
        .await?;

    match response.message {
        OpfsOperationOutput::DownloadUrl(url) => Some(url),
        _ => None
    }
}

/// Run CoreOperation and return the CoreOperationOutput
#[wasm_bindgen]
pub async fn execute_operation(effect: Uint8Array) -> Uint8Array {
    let executor = DiContainer::get_instance().get_native_executor().await;
    let effect: CoreOperation = deserialize(&effect);
    let output = executor.handle(u32::MAX, effect).await;
    serialize(&output)
}

/// Run CoreOperation and call core to handle response
/// Return the next Operations that need to execute.
#[wasm_bindgen]
pub async fn execute(request_id: u32, effect: Uint8Array) -> Uint8Array {
    let executor = DiContainer::get_instance().get_native_executor().await;
    let effect: CoreOperation = deserialize(&effect);
    let output = executor.handle(request_id, effect).await;
    handle_response(request_id, serialize(&output)).await
}

pub fn deserialize<E: Serialize>(data: &Uint8Array) -> E
where
    E: for<'de> Deserialize<'de>
{
    let vec = data.to_vec();
    let options = bincode_options();
    let mut deser = bincode::Deserializer::from_slice(vec.as_slice(), options);
    let mut deserializer = <dyn erased_serde::Deserializer>::erase(&mut deser);
    let data: E = erased_serde::deserialize(&mut deserializer).expect("Failed to deserialize effect");
    data
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

