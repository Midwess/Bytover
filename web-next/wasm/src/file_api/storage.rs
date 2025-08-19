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
use crate::browser_cache::cache::BrowserCache;
use crate::local_resource_path::WebExtLocalResourcePath;

#[derive(Clone)]
pub struct WasmFile(pub File);

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
    db: PoolRequest<NeverSend<Database>>,
    thumbnail_cache: BrowserCache
}

impl FileStorage {
    pub fn new(db: PoolRequest<NeverSend<Database>>, thumbnail_cache: BrowserCache) -> FileStorage {
        FileStorage {
            device_files: Arc::new(Mutex::new(HashMap::new())),
            thumbnail_cache,
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

    pub(crate) async fn get_file_by_path(&self, path: &LocalResourcePath) -> Option<File> {
        let LocalResourcePath::PlatformIdentifier(platform_identifier) = path else {
            return None;
        };

        let resource_id = platform_identifier.split_once("device://")?.1.to_string().parse::<u64>().ok()?;
        self.get_file(resource_id).await
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
        let thumbnail_cache_key = resource_id.to_string();
        let data = self.thumbnail_cache.get(thumbnail_cache_key.as_str(), false).await.ok()??;
        Some(data)
    }

    pub(crate) async fn save_thumbnail(&self, resource_id: u64, png_bytes: Vec<u8>) -> Option<LocalResourcePath> {
        let key = resource_id.to_string();

        self.thumbnail_cache.put(&key, png_bytes).await.ok()?;

        Some(LocalResourcePath::cache(&self.thumbnail_cache.name, key))
    }
}
