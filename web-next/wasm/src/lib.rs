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
pub mod webrtc;

// /shared/src/lib.rs
use crate::di_container::DiContainer;
use crate::file_system::device_file::{wasm_file, DeviceFile};
use crate::file_system::io::OPFS_WORKER;
use crate::file_system::path_extension::WebExtLocalResourcePath;
use crate::web_worker::bridge::{WebWorkerBridge, WorkerMessage};
use crate::web_worker::core::{CoreWorker, CoreWorkerOperation};
use crate::web_worker::opfs::{FileOperation, OpfsOperation, OpfsOperationOutput};
use bincode::{DefaultOptions, Options};
use core_services::logger;
use core_services::utils::never_send::NeverSend;
use core_services::wasm::extensions::VecExtension;
use core_services::wasm::HttpClient;
pub use crux_core::bridge::Bridge;
pub use crux_core::{Core, Request};
use devlog_sdk::distributed_id::gen_id;
use erased_serde::Serialize;
use js_sys::{Array, Promise};
use n0_future::task;
use serde::Deserialize;
use shared::app::shelf::module::ResourceSelection;
use shared::entities::local_resource::{LocalResource, LocalResourcePath};
use shared::shell::api::{CoreRequest, CruxRequest};
use shared::CoreOperation;
use std::collections::HashSet;
use std::sync::LazyLock;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::Uint8Array;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, File, LockMode, LockOptions};

static CORE_WORKER: LazyLock<NeverSend<WebWorkerBridge<CoreWorker>>> =
    LazyLock::new(|| NeverSend(WebWorkerBridge::spawn("core-worker")));

const LOCK_NAME_PREFIX: &str = "bitbridge_session_";
const LEGACY_STORAGE_PREFIX: &str = "bitbridge_storage_session_";
const OPFS_SESSION_PREFIX: &str = "session-";

fn hold_session_lock_forever(lock_name: &str) -> Result<(), JsValue> {
    let window = window().ok_or_else(|| JsValue::from_str("no window"))?;
    let locks = window.navigator().locks();

    let options = LockOptions::new();
    options.set_mode(LockMode::Exclusive);

    let callback = Closure::wrap(Box::new(|_lock: JsValue| -> Promise {
        Promise::new(&mut |_resolve, _reject| {})
    }) as Box<dyn FnMut(JsValue) -> Promise>);

    let _ = locks.request_with_options_and_callback(
        lock_name,
        &options,
        callback.as_ref().unchecked_ref(),
    );

    callback.forget();
    Ok(())
}

async fn query_held_session_uuids() -> HashSet<String> {
    let mut set = HashSet::new();
    let Some(w) = window() else {
        return set;
    };
    let locks = w.navigator().locks();

    let Ok(snapshot) = JsFuture::from(locks.query()).await else {
        return set;
    };

    let Ok(held_val) = js_sys::Reflect::get(&snapshot, &JsValue::from_str("held")) else {
        return set;
    };

    let held_array = Array::from(&held_val);
    for entry in held_array.iter() {
        let Ok(name_val) = js_sys::Reflect::get(&entry, &JsValue::from_str("name")) else {
            continue;
        };
        let Some(name) = name_val.as_string() else {
            continue;
        };
        if let Some(uuid) = name.strip_prefix(LOCK_NAME_PREFIX) {
            set.insert(uuid.to_string());
        }
    }
    set
}

async fn list_orphan_session_paths() -> Vec<String> {
    let held = query_held_session_uuids().await;

    let Some(response) = OPFS_WORKER
        .send(WorkerMessage::new(OpfsOperation {
            file_path: "/".to_owned(),
            operation: FileOperation::ListSessions,
        }))
        .await
    else {
        return Vec::new();
    };

    let OpfsOperationOutput::Sessions(dirs) = response.message else {
        return Vec::new();
    };

    let mut orphans = Vec::new();
    for dir in dirs {
        let Some(uuid) = dir.strip_prefix(OPFS_SESSION_PREFIX) else {
            continue;
        };
        if !held.contains(uuid) {
            orphans.push(dir);
        }
    }
    orphans
}

fn purge_legacy_heartbeat_keys() {
    let Some(w) = window() else {
        return;
    };
    let Ok(Some(storage)) = w.local_storage() else {
        return;
    };
    let len = storage.length().unwrap_or(0);
    let mut keys = Vec::new();
    for i in 0..len {
        let Ok(Some(key)) = storage.key(i) else {
            continue;
        };
        if key.starts_with(LEGACY_STORAGE_PREFIX) {
            keys.push(key);
        }
    }
    for key in keys {
        let _ = storage.remove_item(&key);
    }
}

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
        return Uint8Array::default();
    };

    response.message.0
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn handle_response(id: u32, data: Uint8Array) -> Uint8Array {
    let Some(response) = CORE_WORKER.send(WorkerMessage::new(CoreWorkerOperation::HandleResponse(id, data))).await else {
        return Uint8Array::default();
    };

    response.message.0
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub async fn view() -> Uint8Array {
    let Some(response) = CORE_WORKER.send(WorkerMessage::new(CoreWorkerOperation::View)).await else {
        return Uint8Array::default();
    };

    response.message.0
}

#[wasm_bindgen]
pub async fn is_compatible() -> bool {
    log::info!("Checking is compatible");
    let Some(with_browser) = window() else {
        log::info!("No window");
        return false;
    };

    if with_browser.is_null() || with_browser.is_undefined() {
        log::info!("Window is null");
        return false;
    }

    let Ok(with_cache) = with_browser.caches() else {
        log::info!("No caches");
        return false;
    };

    if with_cache.is_null() || with_cache.is_undefined() {
        log::info!("Caches is null");
        return false;
    }

    let navigator = with_browser.navigator();
    let storage = navigator.storage();

    if storage.is_null() || storage.is_undefined() {
        log::info!("Storage is null");
        return false;
    }

    if !js_sys::Reflect::has(&navigator, &JsValue::from_str("locks")).unwrap_or(false) {
        log::info!("No navigator.locks");
        return false;
    }

    true
}

#[wasm_bindgen]
pub async fn init() {
    logger::setup();

    let session_id = uuid::Uuid::new_v4().to_string();
    log::info!("Storage session initialized: {}", session_id);

    let lock_name = format!("{}{}", LOCK_NAME_PREFIX, session_id);
    if let Err(e) = hold_session_lock_forever(&lock_name) {
        log::error!("Failed to acquire session lock {}: {:?}", lock_name, e);
        return;
    }

    let di_container = DiContainer::get_instance();
    di_container.init().await;

    let init_msg = WorkerMessage::new(OpfsOperation {
        file_path: "/".to_owned(),
        operation: FileOperation::Init {
            storage_session_id: session_id.clone(),
        },
    });
    OPFS_WORKER.send(init_msg).await;

    task::spawn(async move {
        purge_legacy_heartbeat_keys();
        let orphans = list_orphan_session_paths().await;
        if !orphans.is_empty() {
            log::info!("Reaping {} orphan session workspaces", orphans.len());
            let cleanup_msg = WorkerMessage::new(OpfsOperation {
                file_path: "/".to_owned(),
                operation: FileOperation::CleanUp { paths: orphans },
            });
            OPFS_WORKER.send(cleanup_msg).await;
        }
    });
}

#[wasm_bindgen]
pub async fn force_cleanup() {
    let orphans = list_orphan_session_paths().await;
    log::info!("force_cleanup: {} orphan(s)", orphans.len());
    if orphans.is_empty() {
        return;
    }
    let _ = OPFS_WORKER
        .send(WorkerMessage::new(OpfsOperation {
            file_path: "/".to_owned(),
            operation: FileOperation::CleanUp { paths: orphans },
        }))
        .await;
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
                operation: FileOperation::AddFile(file),
            }))
            .await;

        let OpfsOperationOutput::LocalResourceInstance(resource_instance) = resp.unwrap().message else {
            continue;
        };

        let resource_instance: LocalResource = deserialize(&resource_instance);
        paths.push(ResourceSelection {
            path: resource_instance.path,
            r#type: Some(resource_instance.r#type),
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
                files: files.into_iter().map(wasm_file).collect(),
            },
        }))
        .await;

    let OpfsOperationOutput::LocalResourceInstance(resource_instance) = resp.unwrap().message else {
        return Uint8Array::default();
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
            operation: FileOperation::GetFile,
        }))
        .await
        .unwrap()
        .message;

    let OpfsOperationOutput::File(file) = resp else {
        log::info!("No file at {:?}", path);
        return None;
    };

    Some(file)
}

#[wasm_bindgen]
pub async fn get_download_url(path: Uint8Array) -> Option<String> {
    let path: LocalResourcePath = deserialize(&path);
    let opfs_path = match path {
        LocalResourcePath::AbsolutePath(path) => return Some(path),
        LocalResourcePath::PlatformIdentifier(_) => path.opfs_path()?,
        _ => return None,
    };

    let _ = OPFS_WORKER
        .send(WorkerMessage::new(OpfsOperation {
            file_path: opfs_path.clone(),
            operation: FileOperation::Open,
        }))
        .await;

    let response = OPFS_WORKER
        .send(WorkerMessage::new(OpfsOperation {
            file_path: opfs_path,
            operation: FileOperation::GenerateSource,
        }))
        .await?;

    match response.message {
        OpfsOperationOutput::DownloadUrl(url) => Some(url),
        _ => None,
    }
}

pub fn is_browser_support_duplex() -> bool {
    let Some(_) = window() else {
        log::info!("No window");
        return false;
    };

    HttpClient::is_support_duplex_stream()
}

/// Run CoreOperation and return the CoreOperationOutput
#[wasm_bindgen]
pub async fn execute_operation(effect: Uint8Array) -> Uint8Array {
    let executor = DiContainer::get_instance().get_native_executor().await;
    let bridge = DiContainer::get_instance().core_bridge();
    let effect: CoreOperation = deserialize(&effect);
    let output = executor.handle(CoreRequest::new(CruxRequest::Id(0), bridge), effect).await;
    serialize(&output)
}

/// Create file at path
#[wasm_bindgen]
pub async fn create_file(file_path: Uint8Array, data: Uint8Array) {
    let path: LocalResourcePath = deserialize(&file_path);
    let _ = OPFS_WORKER
        .send(WorkerMessage::new(OpfsOperation {
            file_path: path.opfs_path().unwrap(),
            operation: FileOperation::WriteNew { data },
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
    let request = CoreRequest::new(CruxRequest::Id(request_id), bridge);
    let output = executor.handle(request.clone(), effect).await;
    handle_response(request_id, serialize(&output)).await
}

pub fn deserialize<E: Serialize>(data: &Uint8Array) -> E
where
    E: for<'de> Deserialize<'de>,
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
    DefaultOptions::new().with_fixint_encoding().allow_trailing_bytes()
}
