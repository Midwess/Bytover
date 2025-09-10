use std::ops::Deref;
use gloo::worker::{HandlerId, Worker, WorkerScope};
use js_sys::{Array, Uint8Array};
use n0_future::task::spawn;
use serde::{Deserialize, Serialize};
use shared::CoreOperation;
use crate::{deserialize, serialize};
use crate::di_container::DiContainer;
use crate::executor::executor::NativeExecutor;
use crate::file_api::storage::FileStorage;
use crate::web_worker::main::WorkerMessage;
use wasm_bindgen_futures::JsFuture;
use web_sys::File;
use shared::app::file_system::file::LocalResourcePath;
use std::sync::Arc;
use crate::file_api::path_extension::WebExtLocalResourcePath;
use crate::file_api::file_extension::VecExtension;

#[derive(Serialize, Deserialize)]
pub enum NativeExecutorOperation {
    HandleEffect(u32, #[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    Init,
    AddDeviceFiles(#[serde(with = "serde_wasm_bindgen::preserve")] Array),
    GetDeviceFile(u64),
    LoadThumbnailBytes(u64),
    LoadThumbnailSource(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    DownloadFileFromCache {
        #[serde(with = "serde_wasm_bindgen::preserve")]
        path: Uint8Array,
        #[serde(with = "serde_wasm_bindgen::preserve")]
        writer: web_sys::FileSystemWritableFileStream
    },
    Execute(u32, #[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
}

unsafe impl Send for NativeExecutorOperation {}
unsafe impl Sync for NativeExecutorOperation {}

#[derive(Serialize, Deserialize)]
pub enum NativeExecutorOutput {
    Effects(u32, #[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    Void,
    ThumbnailBytes(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    ThumbnailSource(Option<String>),
    DeviceFile(#[serde(with = "serde_wasm_bindgen::preserve")] File),
    DeviceFiles(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    ExecuteResult(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    
    // Exhausted events
    ForwardCoreOperationOutput(u32, #[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    UpdateAppEvent(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
}

unsafe impl Send for NativeExecutorOutput {}
unsafe impl Sync for NativeExecutorOutput {}

pub struct ExecutingWorker {
    storage: FileStorage,
    native_executor: &'static NativeExecutor,
}

impl Worker for ExecutingWorker {
    type Message = ();
    type Input = WorkerMessage<NativeExecutorOperation>;
    type Output = WorkerMessage<NativeExecutorOutput>;

    fn create(scope: &WorkerScope<Self>) -> Self {
        log::info!("Creating worker");

        let di_instance = DiContainer::get_instance();

        Self {
            storage: di_instance.file_storage(),
            native_executor: di_instance.get_native_executor()
        }
    }

    fn update(&mut self, scope: &WorkerScope<Self>, msg: Self::Message) {
        log::info!("Received message: {:?}", msg);
    }

    fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, id: HandlerId) {
        let scope = scope.clone();
        let native_executor = self.native_executor;
        let storage = self.storage.clone();
        spawn(async move {
            match msg.deref() {
                NativeExecutorOperation::HandleEffect(request_id, data) => {
                    let effect: CoreOperation = deserialize(&data);
                    let output = native_executor.handle(*request_id, effect).await;
                    scope.respond(id, WorkerMessage::new(NativeExecutorOutput::Effects(*request_id, serialize(&output))));
                }
                NativeExecutorOperation::Init => {
                    let di_container = DiContainer::get_instance();
                    di_container.init(Arc::new(crate::ShellRuntime {})).await;
                    scope.respond(id, WorkerMessage::new(NativeExecutorOutput::Void));
                }
                NativeExecutorOperation::AddDeviceFiles(files) => {
                    let paths = storage.add(files).await;
                    scope.respond(id, WorkerMessage::new(NativeExecutorOutput::DeviceFiles(serialize(&paths))));
                }
                NativeExecutorOperation::GetDeviceFile(resource_id) => {
                    let file = storage.get(*resource_id).await;
                    let result = file.map(|it| it.file.0);
                    if let Some(result) = result {
                        scope.respond(id, WorkerMessage::new(NativeExecutorOutput::DeviceFile(result)));
                    }
                    else {
                        scope.respond(id, WorkerMessage::new(NativeExecutorOutput::Void));
                    }
                }
                NativeExecutorOperation::LoadThumbnailBytes(resource_id) => {
                    let repository = DiContainer::get_instance().get_local_resource_repository();
                    let path = LocalResourcePath::cache("thumbnails", resource_id.to_string());
                    let Ok(mut reader) = repository.read(path, 1024 * 256).await else {
                        scope.respond(id, WorkerMessage::new(NativeExecutorOutput::ThumbnailBytes(Uint8Array::default())));
                        return;
                    };
                    let Ok(data) = reader.read_all().await else {
                        scope.respond(id, WorkerMessage::new(NativeExecutorOutput::ThumbnailBytes(Uint8Array::default())));
                        return;
                    };
                    scope.respond(id, WorkerMessage::new(NativeExecutorOutput::ThumbnailBytes(data.into_uint_array())))
                }
                NativeExecutorOperation::LoadThumbnailSource(path_data) => {
                    let path_vec = path_data.to_vec();
                    let options = crate::bincode_options();
                    let mut deser = bincode::Deserializer::from_slice(&path_vec, options);
                    let mut deserializer = <dyn erased_serde::Deserializer>::erase(&mut deser);
                    let path: LocalResourcePath = erased_serde::deserialize(&mut deserializer).expect("Failed to deserialize effect");
                    
                    if let LocalResourcePath::AbsolutePath(path) = path {
                        scope.respond(id, WorkerMessage::new(NativeExecutorOutput::ThumbnailSource(Some(path))));
                        return;
                    }
                    
                    let Some(resource_id) = path.thumbnail_resource_id() else {
                        scope.respond(id, WorkerMessage::new(NativeExecutorOutput::ThumbnailSource(None)));
                        return;
                    };
                    
                    let repository = DiContainer::get_instance().get_local_resource_repository();
                    let path = LocalResourcePath::cache("thumbnails", resource_id.to_string());
                    let Ok(mut reader) = repository.read(path, 1024 * 256).await else {
                        scope.respond(id, WorkerMessage::new(NativeExecutorOutput::ThumbnailSource(None)));
                        return;
                    };
                    let Ok(data) = reader.read_all().await else {
                        scope.respond(id, WorkerMessage::new(NativeExecutorOutput::ThumbnailSource(None)));
                        return;
                    };
                    
                    let blob_options = web_sys::BlobPropertyBag::new();
                    blob_options.set_type("image/png");
                    
                    let parts = Array::new();
                    parts.push(&data.into_uint_array());
                    
                    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &blob_options) else {
                        scope.respond(id, WorkerMessage::new(NativeExecutorOutput::ThumbnailSource(None)));
                        return;
                    };
                    
                    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
                        scope.respond(id, WorkerMessage::new(NativeExecutorOutput::ThumbnailSource(None)));
                        return;
                    };
                    
                    scope.respond(id, WorkerMessage::new(NativeExecutorOutput::ThumbnailSource(Some(url))))
                }
                NativeExecutorOperation::DownloadFileFromCache { path, writer } => {
                    let path_vec = path.to_vec();
                    let options = crate::bincode_options();
                    let mut deser = bincode::Deserializer::from_slice(&path_vec, options);
                    let mut deserializer = <dyn erased_serde::Deserializer>::erase(&mut deser);
                    let path: LocalResourcePath = erased_serde::deserialize(&mut deserializer).expect("Failed to deserialize effect");
                    let repository = DiContainer::get_instance().get_local_resource_repository();
                    let Ok(mut reader) = repository.read(path, 1024 * 256).await else {
                        let _ = JsFuture::from(writer.close()).await;
                        scope.respond(id, WorkerMessage::new(NativeExecutorOutput::Void));
                        return;
                    };
                    
                    while let Some(data) = reader.next().await.unwrap() {
                        let Ok(fut) = writer.write_with_u8_array(&data) else {
                            break;
                        };
                        
                        if let Err(e) = JsFuture::from(fut).await {
                            log::error!("Failed to write to file: {:?}", e);
                            break;
                        }
                    }
                    
                    let _ = JsFuture::from(writer.close()).await;
                    scope.respond(id, WorkerMessage::new(NativeExecutorOutput::Void))
                }
                NativeExecutorOperation::Execute(request_id, effect) => {
                    let effect: CoreOperation = deserialize(&effect);
                    let output = native_executor.handle(*request_id, effect).await;
                    let result = crate::handle_response(*request_id, serialize(&output)).await;
                    scope.respond(id, WorkerMessage::new(NativeExecutorOutput::ExecuteResult(result)))
                }
            }
        });
    }
}
