use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use futures::lock::Mutex;
use web_sys::File;
use web_sys::js_sys::Array;
use devlog_sdk::distributed_id::gen_id;
use shared::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};

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

#[derive(Debug, Clone)]
pub struct FileStorage {
    device_files: Arc<Mutex<HashMap<u64, WasmFile>>>,
}

impl FileStorage {
    pub fn new() -> FileStorage {
        FileStorage {
            device_files: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_device_wasm_files(&self, files: &Array) -> Vec<LocalResourcePath> {
        let mut device_files = self.device_files.lock().await;
        let mut returned_paths = vec![];
        for file in files.iter() {
            let f = File::from(file);
            let id = gen_id().await;
            device_files.insert(id, WasmFile(f));
            let path = LocalResourcePath::PlatformIdentifier(format!("device://{}", id));
            returned_paths.push(path);
        }

        returned_paths
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
            LocalResourcePath::PlatformIdentifier(path) => {
                path.split_once("device://")?.1.to_string().parse::<u64>().ok()?
            },
            _ => return None,
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

    pub(crate) async fn save(&self) {

    }
}
