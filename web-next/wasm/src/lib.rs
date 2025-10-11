extern crate core;

pub mod bridge;
pub mod config;
pub mod di_container;
mod errors;
pub mod executor;
pub mod file_system;
pub mod network;
pub mod repository;
pub mod web_worker;

// /shared/src/lib.rs
use crate::di_container::DiContainer;
use crate::file_system::device_file::{wasm_file, DeviceFile};
use crate::file_system::io::OPFS_WORKER;
use crate::file_system::path_extension::WebExtLocalResourcePath;
use crate::web_worker::bridge::{WebWorkerBridge, WorkerMessage};
use crate::web_worker::core::{CoreWorker, CoreWorkerOperation};
use crate::web_worker::opfs::{FileOperation, OpfsOperation, OpfsOperationOutput};
use bincode::Options;
use core_services::logger;
use core_services::utils::never_send::NeverSend;
use core_services::wasm::extensions::VecExtension;
use core_services::wasm::HttpClient;
pub use crux_core::bridge::Bridge;
pub use crux_core::{Core, Request};
use devlog_sdk::distributed_id::gen_id;
use erased_serde::Serialize;
use js_sys::{Array, Promise};
use serde::Deserialize;
use shared::app::shelf::module::ResourceSelection;
use shared::entities::local_resource::{LocalResource, LocalResourcePath};
use shared::shell::api::CoreRequest;
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
    log::info!("Checking is compatible");
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
    logger::setup();
    log::info!("Initializing");
    let di_container = DiContainer::get_instance();
    di_container.init().await;
}

/// Add device files to opfs
/// and return list of ResourceSelections
#[wasm_bindgen]
pub async fn add_device_files(files: &Array) -> Uint8Array {
    let mut paths = vec![];
    for file in files.iter() {
        let file: File = file.dyn_into().unwrap();
        let file = DeviceFile::new(file).await;
        let resp = OPFS_WORKER
            .send(WorkerMessage::new(OpfsOperation {
                file_path: file.local_resource().path.opfs_path().unwrap(),
                operation: FileOperation::AddFile(file)
            }))
            .await;

        let OpfsOperationOutput::LocalResourceInstance(resource_instance) = resp.unwrap().message else {
            continue;
        };

        let resource_instance: LocalResource = deserialize(&resource_instance);
        paths.push(ResourceSelection {
            path: resource_instance.path,
            r#type: Some(resource_instance.r#type)
        });
    }

    serialize(&paths)
}

#[wasm_bindgen]
pub async fn add_device_folder(path: String, files: Vec<File>) -> Uint8Array {
    let folder_path = LocalResourcePath::device_file(gen_id().await);
    let resp = OPFS_WORKER
        .send(WorkerMessage::new(OpfsOperation {
            file_path: folder_path.opfs_path().unwrap(),
            operation: FileOperation::AddFolder {
                path,
                files: files.into_iter().map(wasm_file).collect()
            }
        }))
        .await;

    let OpfsOperationOutput::LocalResourceInstance(resource_instance) = resp.unwrap().message else {
        return Uint8Array::default()
    };

    let resource_instance: LocalResource = deserialize(&resource_instance);

    serialize(&resource_instance.path)
}

#[wasm_bindgen]
pub async fn get_device_file(path: Uint8Array) -> Option<File> {
    let path: LocalResourcePath = deserialize(&path);
    let resp = OPFS_WORKER
        .send(WorkerMessage::new(OpfsOperation {
            file_path: path.opfs_path().unwrap(),
            operation: FileOperation::GetFile
        }))
        .await
        .unwrap()
        .message;

    let OpfsOperationOutput::File(file) = resp else {
        log::info!("No file at {:?}", path);
        return None
    };

    Some(file)
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

pub fn is_browser_support_duplex() -> bool {
    let Some(_) = window() else {
        log::info!("No window");
        return false
    };

    HttpClient::is_support_duplex_stream()
}

/// Run CoreOperation and return the CoreOperationOutput
#[wasm_bindgen]
pub async fn execute_operation(effect: Uint8Array) -> Uint8Array {
    let executor = DiContainer::get_instance().get_native_executor().await;
    let bridge = DiContainer::get_instance().core_bridge();
    let effect: CoreOperation = deserialize(&effect);
    let output = executor.handle(CoreRequest::new(0, bridge), effect).await;
    serialize(&output)
}

/// Create file at path
#[wasm_bindgen]
pub async fn create_file(file_path: Uint8Array, data: Uint8Array) {
    let path: LocalResourcePath = deserialize(&file_path);
    let _ = OPFS_WORKER
        .send(WorkerMessage::new(OpfsOperation {
            file_path: path.opfs_path().unwrap(),
            operation: FileOperation::WriteNew { data }
        }))
        .await;
}

/// Run CoreOperation and call core to handle response
/// Return the next Operations that need to execute.
#[wasm_bindgen]
pub async fn execute(request_id: u32, effect: Uint8Array) -> Uint8Array {
    let executor = DiContainer::get_instance().get_native_executor().await;
    let bridge = DiContainer::get_instance().core_bridge();
    let effect: CoreOperation = deserialize(&effect);
    let request = CoreRequest::new(request_id, bridge);
    let output = executor.handle(request.clone(), effect).await;
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
