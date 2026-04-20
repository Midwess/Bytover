use crate::file_system::io::IOReaderBlobImpl;
use crate::file_system::path_extension::WebExtLocalResourcePath;
use crate::{deserialize, serialize};
use async_stream::stream;
use chrono::Utc;
use core_services::local_storage::entry::FileEntry;
use core_services::local_storage::stream::IOCursor;
use core_services::local_storage::zip::ZipStream;
use core_services::wasm::extensions::FileExtension;
use devlog_sdk::distributed_id::gen_id;
use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};
use shared::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use std::cell::OnceCell;
use std::fmt::Debug;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use web_sys::{Blob, File};

pub fn wasm_file(file: File) -> WebFile {
    WebFile {
        webkit_path: file.webkit_path(),
        file,
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WebFile {
    #[serde(with = "serde_wasm_bindgen::preserve")]
    pub file: File,
    pub webkit_path: Option<String>,
}

impl Debug for WebFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "wasm_file {{ name: {}, size: {} }}", self.name(), self.size())
    }
}

unsafe impl Send for WebFile {}

unsafe impl Sync for WebFile {}

impl WebFile {
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

impl Deref for WebFile {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

pub struct DeviceFolder {
    pub(crate) files: Arc<Vec<WebFile>>,
    pub(crate) base_path: String,
    pub(crate) resource_instance: LocalResource,
}

impl DeviceFolder {
    pub async fn new(local_resource_path: LocalResourcePath, path: PathBuf, files: Vec<WebFile>) -> Self {
        let mut total_size = 0u64;
        for file in files.iter() {
            total_size += file.size() as u64;
        }

        let resource_instance = LocalResource {
            name: path.file_name().unwrap().to_str().unwrap().to_string(),
            size: total_size,
            order_id: local_resource_path.device_file_id().unwrap(),
            path: local_resource_path,
            thumbnail_path: None,
            r#type: ResourceType::Folder,
            shelf_id: 0,
        };

        Self {
            files: Arc::new(files),
            base_path: path.to_str().unwrap().to_string(),
            resource_instance,
        }
    }

    pub async fn cursor(&self, buffer_size: usize) -> anyhow::Result<Box<dyn IOCursor>> {
        let files = self.files.clone();
        let stream = stream! {
            for file in files.iter() {
                let writer = Box::new(IOReaderBlobImpl::from_file(file, buffer_size.min(file.size() as usize)).await?);
                yield Ok::<_, anyhow::Error>(writer as Box<dyn IOCursor>);
            }
        };

        let entry = FileEntry {
            is_dir: false,
            modified_at: Utc::now().into(),
            size: self.resource_instance.size,
            path: self.base_path.clone().into(),
        };

        Ok(Box::new(
            ZipStream::new_from_stream(Box::pin(stream), entry, buffer_size).await?,
        ))
    }
}

/// Keep track of files that are being chosen by the user from their device.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DeviceFile {
    pub(crate) file: WebFile,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    pub(crate) resource: Uint8Array,

    #[serde(skip)]
    pub(crate) resource_instance: OnceCell<LocalResource>,
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

        let resource_instance = LocalResource {
            name: file.name(),
            size: file.size() as u64,
            path: LocalResourcePath::device_file(order_id),
            thumbnail_path: None,
            r#type: resource_type,
            order_id,
            shelf_id: 0,
        };

        let resource = serialize(&resource_instance);
        let resource_cell = OnceCell::new();
        resource_cell.set(resource_instance).unwrap();

        Self {
            file: wasm_file(file),
            resource_instance: resource_cell,
            resource,
        }
    }

    pub fn local_resource(&self) -> &LocalResource {
        if let Some(resource) = self.resource_instance.get() {
            return resource;
        }

        let _ = self.resource_instance.set(deserialize(&self.resource));

        self.resource_instance.get().unwrap()
    }

    pub fn raw_local_resource(&self) -> &Uint8Array {
        &self.resource
    }

    pub fn blob(self) -> Option<Blob> {
        (self.file.file).dyn_into().ok()
    }
}
