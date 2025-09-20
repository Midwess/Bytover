use crate::file_api::device_file::FileStorage;
use crate::file_api::opfs::OPFS_WORKER;
use crate::file_api::path_extension::WebExtLocalResourcePath;
use crate::web_worker::bridge::WorkerMessage;
use crate::web_worker::opfs::{FileOperation, OpfsOperation, OpfsOperationOutput};
use anyhow::anyhow;
use core_services::wasm::{HttpClient, XhrEvent};
use futures::channel::mpsc;
use futures::StreamExt;
use futures_channel::mpsc::Receiver;
use n0_future::task::{spawn, JoinHandle};
use n0_future::SinkExt;
use shared::app::repository::local_resource::LocalResourceRepository;
use shared::core_api::{NetStream, NetStreamEvent, NetStreamInner, UploadRequest, UploadResponse};
use shared::entities::file_system::file::LocalResourcePath;
use std::sync::Arc;
use wasm_bindgen::JsValue;
use web_sys::Blob;

pub struct NetStreamImpl {
    pub storage: FileStorage,
    pub resource_repo: Arc<dyn LocalResourceRepository>
}

pub struct NetStreamInnerImpl {
    storage: FileStorage,
    resource_repo: Arc<dyn LocalResourceRepository>,
    requests: Vec<UploadRequest>,
    path: LocalResourcePath,
    handle: Option<JoinHandle<Result<(), JsValue>>>
}

impl NetStreamImpl {
    pub fn new(storage: FileStorage, resource_repo: Arc<dyn LocalResourceRepository>) -> Self {
        Self { storage, resource_repo }
    }
}

#[async_trait::async_trait(?Send)]
impl NetStream for NetStreamImpl {
    async fn upload_resource(&self, requests: Vec<UploadRequest>, path: LocalResourcePath) -> anyhow::Result<Box<dyn NetStreamInner>> {
        Ok(Box::new(NetStreamInnerImpl {
            storage: self.storage.clone(),
            resource_repo: self.resource_repo.clone(),
            requests,
            path,
            handle: None
        }))
    }
}

#[async_trait::async_trait(?Send)]
impl NetStreamInner for NetStreamInnerImpl {
    async fn start(&mut self) -> anyhow::Result<Receiver<NetStreamEvent>> {
        let (mut tx, rx) = mpsc::channel::<NetStreamEvent>(20);
        let Some(blob) = self.get_blob().await else {
            return Err(anyhow!("No blob to upload"));
        };

        let mut all_requests = self.requests.drain(..).collect::<Vec<_>>();
        self.handle = Some(spawn(async move {
            let mut current_position = 0;
            let mut responses = Vec::new();
            let result: Result<(), JsValue> = 'upload: loop {
                let Some(request) = all_requests.pop() else { break 'upload Ok(()) };

                let new_position = match request.x_content_length {
                    Some(x) => (current_position + x).min(blob.size() as u64),
                    None => blob.size() as u64
                };

                let content_length = new_position - current_position;
                log::info!("Uploading {}", content_length);
                if content_length == 0 {
                    break 'upload Ok(())
                }

                let next_blob = blob.slice_with_f64_and_f64(current_position as f64, new_position as f64)?;
                current_position = new_position;

                let mut request = HttpClient::new()
                    .url(request.url.as_str())
                    .header("content-type", "application/octet-stream")
                    .method("PUT")
                    .body_blob(next_blob)
                    .xhr()?;

                'event_loop: while let Some(event) = request.next_event().await {
                    log::info!("XHR event: {:?}", event);
                    match event {
                        XhrEvent::Error(value) => {
                            break 'upload Err(value);
                        }
                        XhrEvent::Complete { headers, body } => {
                            let body: Option<serde_json::Value> = body.as_string().and_then(|s| serde_json::from_str(&s).ok());
                            let response = UploadResponse { headers, body };
                            responses.push(response);
                            break 'event_loop
                        }
                        XhrEvent::InProgress(value) => {
                            let total_bytes = value.total();
                            let _ = tx.try_send(NetStreamEvent::Progress {
                                uploaded_bytes: total_bytes as u64
                            });
                        }
                    };
                }
            };

            let end_event = match result {
                Ok(()) => NetStreamEvent::Completed(responses),
                Err(value) => NetStreamEvent::Error(anyhow!("Upload failed: {:?}", value))
            };

            let _ = tx.try_send(end_event);
            let _ = tx.close();
            Ok(())
        }));

        Ok(rx)
    }

    async fn end(&mut self) -> anyhow::Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }

        Ok(())
    }
}

impl NetStreamInnerImpl {
    async fn get_blob(&self) -> Option<Blob> {
        if let Some(opfs_path) = self.path.opfs_path() {
            let Some(resp) = OPFS_WORKER
                .send(WorkerMessage::new(OpfsOperation {
                    file_path: opfs_path,
                    operation: FileOperation::Blob
                }))
                .await
            else {
                return None;
            };

            return match resp.message {
                OpfsOperationOutput::Blob(blob) => Some(blob),
                _ => None
            }
        } else if let Some(device_id) = self.path.device_file_id() {
            return self.storage.get(device_id).await.and_then(|device_file| device_file.blob())
        }

        None
    }
}

impl Drop for NetStreamInnerImpl {
    fn drop(&mut self) {
        let _ = futures::executor::block_on(self.end());
    }
}