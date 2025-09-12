use crate::file_api::path_extension::WebExtLocalResourcePath;
use devlog_sdk::distributed_id::gen_id;
use futures::lock::Mutex;
use shared::app::transfer::file_selection_service::ResourceSelection;
use shared::entities::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use web_sys::js_sys::Array;
use web_sys::File;

#[derive(Clone)]
pub struct WasmFile(pub File);

unsafe impl Send for WasmFile {}

impl WasmFile {
    pub fn resource_type(&self) -> ResourceType {
        let mime_type = mime_guess::from_path(self.name()).first_or_octet_stream();
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
pub struct DeviceFile {
    pub(crate) file: WasmFile,
    pub(crate) resource: LocalResource
}

impl DeviceFile {
    pub async fn new(file: File) -> Self {
        let resource_type = if file.type_().starts_with("image/") {
            ResourceType::Image
        } else if file.type_().starts_with("video/") {
            ResourceType::Video
        } else {
            ResourceType::File
        };

        let order_id = gen_id().await;

        let resource = LocalResource {
            name: file.name(),
            size: file.size() as u64,
            path: LocalResourcePath::device_file(order_id),
            thumbnail_path: None,
            r#type: resource_type,
            order_id
        };

        Self {
            file: WasmFile(file),
            resource
        }
    }
}

#[derive(Clone)]
pub struct FileStorage {
    files: Arc<Mutex<HashMap<u64, DeviceFile>>>
}

impl Default for FileStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStorage {
    pub fn new() -> FileStorage {
        FileStorage {
            files: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub async fn add(&self, files: &Array) -> Vec<ResourceSelection> {
        let mut device_files = self.files.lock().await;
        let mut selections = vec![];
        for file in files.iter() {
            let f = File::from(file);
            let device_file = DeviceFile::new(f).await;

            let selection = ResourceSelection {
                path: device_file.resource.path.clone(),
                r#type: Some(device_file.resource.r#type.clone())
            };

            device_files.insert(device_file.resource.order_id, device_file);

            selections.push(selection);
        }

        selections
    }

    pub(crate) async fn get(&self, id: u64) -> Option<DeviceFile> {
        let device_files = self.files.lock().await;
        device_files.get(&id).cloned()
    }
}
