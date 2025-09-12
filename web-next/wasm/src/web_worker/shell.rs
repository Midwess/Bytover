use gloo::worker::{HandlerId, Worker, WorkerScope};
use js_sys::{Array, Uint8Array};
use n0_future::task::spawn;
use n0_future::time::{Interval, interval};
use serde::{Deserialize, Serialize};
use shared::CoreOperation;
use shared::app::operations::CoreOperationOutput;
use shared::app::AppEvent;
use crate::{deserialize, serialize, CoreRequestId};
use crate::di_container::DiContainer;
use crate::web_worker::bridge::{TrustedWorkerMessage, WorkerMessage};
use wasm_bindgen_futures::JsFuture;
use web_sys::File;
use shared::app::file_system::file::LocalResourcePath;
use std::sync::Arc;
use std::time::Duration;
use futures::lock::Mutex;
use once_cell::sync::OnceCell;
use core_services::utils::never_send::NeverSend;
use crate::config::HostInfo;
use crate::file_api::path_extension::WebExtLocalResourcePath;
use crate::file_api::file_extension::VecExtension;
use crate::web_worker::{CoreOperationEncoded, CoreOperationOutputEncoded};

#[derive(Serialize, Deserialize, Debug)]
pub enum ShellWorkerOperation {
    HandleCoreOperation(CoreRequestId, #[serde(with = "serde_wasm_bindgen::preserve")] CoreOperationEncoded),
    Init(HostInfo),
    AddDeviceFiles(#[serde(with = "serde_wasm_bindgen::preserve")] Array),
    GetDeviceFile(String),
    LoadThumbnailBytes(String),
    LoadThumbnailSource(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    DownloadFileFromCache {
        #[serde(with = "serde_wasm_bindgen::preserve")]
        path: Uint8Array,
        #[serde(with = "serde_wasm_bindgen::preserve")]
        writer: web_sys::FileSystemWritableFileStream,
    }
}

unsafe impl Send for ShellWorkerOperation {}
unsafe impl Sync for ShellWorkerOperation {}

#[derive(Serialize, Deserialize)]
pub enum ShellWorkerOperationOutput {
    CoreOperationOutput(CoreRequestId, #[serde(with = "serde_wasm_bindgen::preserve")] CoreOperationOutputEncoded),
    Void,
    ThumbnailBytes(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    ThumbnailSource(Option<String>),
    DeviceFile(#[serde(with = "serde_wasm_bindgen::preserve")] File),
    DeviceFiles(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),

    // Exhausted events
    ForwardCoreOperationOutput(u32, #[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    UpdateAppEvent(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
}

unsafe impl Send for ShellWorkerOperationOutput {}
unsafe impl Sync for ShellWorkerOperationOutput {}

pub struct ShellRuntime {
    scope: NeverSend<WorkerScope<ShellWorker>>,
    handler_id: OnceCell<HandlerId>
}

impl ShellRuntime {
    pub fn new(scope: WorkerScope<ShellWorker>) -> Self {
        Self { scope: NeverSend(scope), handler_id: OnceCell::new() }
    }

    pub fn forward_core_operation_output(self: Arc<Self>, request_id: u32, output: CoreOperationOutput) {
        let Some(handler_id) = self.handler_id.get() else {
            return;
        };

        let serialized_output = serialize(&output);
        self.scope.respond(handler_id.clone(), WorkerMessage::new(ShellWorkerOperationOutput::ForwardCoreOperationOutput(request_id, serialized_output)));
    }

    pub fn update(self: Arc<Self>, event: AppEvent) {
        let Some(handler_id) = self.handler_id.get() else {
            return;
        };

        let serialized_event = serialize(&event);
        self.scope.respond(handler_id.clone(), WorkerMessage::new(ShellWorkerOperationOutput::UpdateAppEvent(serialized_event)));
    }
}

pub struct ThrottleShellRuntime {
    latest_event: Arc<Mutex<Option<(u32, CoreOperationOutput)>>>,
}

impl ThrottleShellRuntime {
    pub fn new(shell_runtime: Arc<ShellRuntime>, delay: Duration) -> Self {
        let latest_event = Arc::new(Mutex::new(None::<(u32, CoreOperationOutput)>));
        let latest_event_clone = latest_event.clone();
        let shell_runtime_clone = shell_runtime.clone();

        spawn(async move {
            let mut interval: Interval = interval(delay);
            interval.tick().await;

            loop {
                interval.tick().await;

                let event_to_send = {
                    let mut latest = latest_event_clone.lock().await;
                    latest.take()
                };

                if let Some(event) = event_to_send {
                    let _ = shell_runtime_clone.clone().forward_core_operation_output(event.0, event.1);
                }
            }
        });

        Self { latest_event }
    }

    pub async fn send(&self, request_id: u32, event: CoreOperationOutput) {
        let mut latest = self.latest_event.lock().await;
        *latest = Some((request_id, event));
    }
}

pub struct ShellWorker {
    shell_runtime: Arc<ShellRuntime>,
}

impl Worker for ShellWorker {
    type Message = ();
    type Input = WorkerMessage<ShellWorkerOperation>;
    type Output = WorkerMessage<ShellWorkerOperationOutput>;

    fn create(scope: &WorkerScope<Self>) -> Self {
        log::info!("Shell worker created");

        let shell_runtime = Arc::new(ShellRuntime::new(scope.clone()));

        Self {
            shell_runtime,
        }
    }

    fn update(&mut self, _: &WorkerScope<Self>, msg: Self::Message) {}

    fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, handler_id: HandlerId) {
        let scope = scope.clone();
        let shell_runtime = self.shell_runtime.clone();
        let id = msg.id().to_string();
        log::info!("Shell worker received message: {:?}", msg.message);
        spawn(async move {
            match msg.message {
                ShellWorkerOperation::HandleCoreOperation(request_id, data) => {
                    let effect: CoreOperation = deserialize(&data);
                    let native_executor = DiContainer::get_instance().get_native_executor();
                    let output = native_executor.handle(request_id, effect).await;
                    scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::CoreOperationOutput(request_id, serialize(&output))));
                }
                ShellWorkerOperation::Init(host_info) => {
                    let di_container = DiContainer::get_instance();
                    di_container.init(host_info, shell_runtime).await;
                    scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::Void));
                }
                ShellWorkerOperation::AddDeviceFiles(files) => {
                    let storage = DiContainer::get_instance().file_storage();
                    let paths = storage.add(&files).await;
                    scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::DeviceFiles(serialize(&paths))));
                }
                ShellWorkerOperation::GetDeviceFile(resource_id) => {
                    let storage = DiContainer::get_instance().file_storage();
                    let resource_id: u64 = resource_id.parse().unwrap();
                    let file = storage.get(resource_id).await;
                    let result = file.map(|it| it.file.0);
                    if let Some(result) = result {
                        scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::DeviceFile(result)));
                    }
                    else {
                        scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::Void));
                    }
                }
                ShellWorkerOperation::LoadThumbnailBytes(resource_id) => {
                    let repository = DiContainer::get_instance().get_local_resource_repository();
                    let path = LocalResourcePath::cache("thumbnails", resource_id.to_string());
                    let Ok(mut reader) = repository.read(path, 1024 * 256).await else {
                        scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::ThumbnailBytes(Uint8Array::default())));
                        return;
                    };
                    let Ok(data) = reader.read_all().await else {
                        scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::ThumbnailBytes(Uint8Array::default())));
                        return;
                    };
                    scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::ThumbnailBytes(data.into_uint_array())))
                }
                ShellWorkerOperation::LoadThumbnailSource(path_data) => {
                    let path: LocalResourcePath = deserialize(&path_data);

                    if let LocalResourcePath::AbsolutePath(path) = path {
                        scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::ThumbnailSource(Some(path))));
                        return;
                    }

                    let Some(resource_id) = path.thumbnail_resource_id() else {
                        scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::ThumbnailSource(None)));
                        return;
                    };

                    let repository = DiContainer::get_instance().get_local_resource_repository();
                    let path = LocalResourcePath::cache("thumbnails", resource_id.to_string());
                    let Ok(mut reader) = repository.read(path, 1024 * 256).await else {
                        scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::ThumbnailSource(None)));
                        return;
                    };
                    let Ok(data) = reader.read_all().await else {
                        scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::ThumbnailSource(None)));
                        return;
                    };

                    let blob_options = web_sys::BlobPropertyBag::new();
                    blob_options.set_type("image/png");

                    let parts = Array::new();
                    parts.push(&data.into_uint_array());

                    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &blob_options) else {
                        scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::ThumbnailSource(None)));
                        return;
                    };

                    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
                        scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::ThumbnailSource(None)));
                        return;
                    };

                    scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::ThumbnailSource(Some(url))))
                }
                ShellWorkerOperation::DownloadFileFromCache { path, writer } => {
                    let path: LocalResourcePath = deserialize(&path);
                    let repository = DiContainer::get_instance().get_local_resource_repository();
                    let Ok(mut reader) = repository.read(path, 1024 * 256).await else {
                        let _ = JsFuture::from(writer.close()).await;
                        scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::Void));
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
                    scope.respond(handler_id, WorkerMessage::response(id, ShellWorkerOperationOutput::Void))
                }
            }
        });
    }
}
