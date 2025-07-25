use core_services::utils::never_send::NeverSend;
use core_services::utils::pool::request::PoolRequest;
use devlog_sdk::distributed_id::gen_id;
use futures::lock::Mutex;
use idb::{Database, Query, TransactionMode};
use js_sys::Uint8Array;
use shared::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use shared::app::transfer::file_selection_service::ResourceSelection;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::js_sys::Array;
use web_sys::{File, IdbTransactionMode};

#[derive(Clone)]
pub struct WasmFile(File);

unsafe impl Send for WasmFile {}

impl WasmFile {
    pub fn resource_type(&self) -> ResourceType {
        let mime_type = mime_guess::from_path(&self.name()).first_or_octet_stream();
        let resource_type = if mime_type.type_() == mime_guess::mime::IMAGE {
            ResourceType::Image
        } else if mime_type.type_() == mime_guess::mime::VIDEO {
            ResourceType::Video
        } else {
            ResourceType::File
        };

        resource_type
    }
}

impl Deref for WasmFile {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct FileStorage {
    device_files: Arc<Mutex<HashMap<u64, WasmFile>>>,
    db: PoolRequest<NeverSend<Database>>
}

impl FileStorage {
    pub fn new(db: PoolRequest<NeverSend<Database>>) -> FileStorage {
        FileStorage {
            device_files: Arc::new(Mutex::new(HashMap::new())),
            db
        }
    }

    pub async fn add_device_wasm_files(&self, files: &Array) -> Vec<ResourceSelection> {
        let mut device_files = self.device_files.lock().await;
        let mut selections = vec![];
        for file in files.iter() {
            let f = File::from(file);
            let mime_type = f.type_();
            let id = gen_id().await;
            device_files.insert(id, WasmFile(f));
            let path = LocalResourcePath::PlatformIdentifier(format!("device://{}", id));

            let resource_type = if mime_type.starts_with("image/") {
                ResourceType::Image
            } else if mime_type.starts_with("video/") {
                ResourceType::Video
            } else {
                ResourceType::File
            };

            let selection = ResourceSelection {
                path,
                r#type: Some(resource_type)
            };

            selections.push(selection);
        }

        selections
    }

    pub(crate) async fn get_file(&self, id: u64) -> Option<File> {
        let device_files = self.device_files.lock().await;
        device_files.get(&id)?.0.clone().into()
    }

    pub(crate) async fn load(&self, path: LocalResourcePath) -> Option<LocalResource> {
        let LocalResourcePath::PlatformIdentifier(platform_identifier) = &path else {
            return None;
        };

        if platform_identifier.starts_with("device://") {
            return self.find_device_file(path).await
        }

        None
    }

    pub(crate) async fn find_device_file(&self, path: LocalResourcePath) -> Option<LocalResource> {
        let device_files = self.device_files.lock().await;
        let resource_id = match &path {
            LocalResourcePath::PlatformIdentifier(path) => path.split_once("device://")?.1.to_string().parse::<u64>().ok()?,
            _ => return None
        };

        let Some(file) = device_files.get(&resource_id).cloned() else {
            return None;
        };

        let local_resource = LocalResource {
            name: file.name(),
            size: file.size() as u64,
            path,
            thumbnail_path: None,
            r#type: file.resource_type(),
            order_id: resource_id
        };

        Some(local_resource)
    }

    pub(crate) async fn read_thumbnail_bytes(&self, resource_id: u64) -> Option<Uint8Array> {
        let Some(db) = self.db.retrieve().await else {
            return None;
        };

        let tx = db.transaction(&["thumbnail"], TransactionMode::ReadOnly).ok()?;
        let store = tx.object_store("thumbnail").ok()?;
        let key = JsValue::from(format!("{resource_id}"));
        let key = Query::from(key);

        log::info!("Reading thumbnail for resource id: {}", resource_id);
        let value = store.get(key).ok()?.await.ok()??;
        Some(value.dyn_into().ok()?)
    }

    pub(crate) async fn save_thumbnail(&self, resource_id: u64, png_bytes: Vec<u8>) -> Option<LocalResourcePath> {
        let Some(db) = self.db.retrieve().await else {
            log::error!("Failed to get db");
            return None;
        };

        let tx = db.transaction(&["thumbnail"], TransactionMode::ReadWrite).ok()?;
        let store = tx.object_store("thumbnail").ok()?;
        let key = JsValue::from(format!("{resource_id}"));
        let value = Uint8Array::from(&png_bytes[..]);

        store.put(&value, Some(&key)).ok()?;
        tx.commit().ok()?;

        Some(LocalResourcePath::PlatformIdentifier(format!(
            "idb://thumbnail/{}",
            resource_id
        )))
    }
}
