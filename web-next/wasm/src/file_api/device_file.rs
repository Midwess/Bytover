use crate::file_api::path_extension::WebExtLocalResourcePath;
use crate::{deserialize, serialize};
use devlog_sdk::distributed_id::gen_id;
use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};
use shared::entities::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use std::cell::OnceCell;
use std::fmt::Debug;
use std::ops::Deref;
use std::path::PathBuf;
use wasm_bindgen::JsCast;
use web_sys::{Blob, File};

#[derive(Clone, Serialize, Deserialize)]
pub struct WasmFile(#[serde(with = "serde_wasm_bindgen::preserve")] pub File);

impl Debug for WasmFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WasmFile {{ name: {}, size: {} }}", self.name(), self.size())
    }
}

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

pub struct DeviceFolder {
    pub(crate) files: Vec<WasmFile>,
    pub(crate) base_path: String,
    pub(crate) resource_instance: LocalResource
}

impl DeviceFolder {
    pub async fn new(path: PathBuf, files: Vec<WasmFile>) -> Self {
        let order_id = gen_id().await;
        let mut total_size = 0u64;
        for file in files.iter() {
            total_size += file.size() as u64;
        }

        let resource_instance = LocalResource {
            name: path.file_name().unwrap().to_str().unwrap().to_string(),
            size: total_size,
            path: LocalResourcePath::device_file(order_id),
            thumbnail_path: None,
            r#type: ResourceType::Folder,
            order_id
        };

        Self {
            files,
            base_path: path.to_str().unwrap().to_string(),
            resource_instance
        }
    }
}

/// Keep track of files that are being chosen by the user from their device.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DeviceFile {
    pub(crate) file: WasmFile,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    pub(crate) resource: Uint8Array,

    #[serde(skip)]
    pub(crate) resource_instance: OnceCell<LocalResource>
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
            order_id
        };

        let resource = serialize(&resource_instance);
        let resource_cell = OnceCell::new();
        resource_cell.set(resource_instance).unwrap();

        Self {
            file: WasmFile(file),
            resource_instance: resource_cell,
            resource
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
        (self.file.0).dyn_into().ok()
    }
}
