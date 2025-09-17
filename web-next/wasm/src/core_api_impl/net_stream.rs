use std::collections::HashMap;
use crate::file_api::device_file::FileStorage;
use crate::file_api::file_extension::VecExtension;
use crate::file_api::path_extension::WebExtLocalResourcePath;
use anyhow::anyhow;
use core_services::utils::never_send::NeverSend;
use futures::channel::mpsc;
use futures_channel::mpsc::UnboundedReceiver;
use shared::app::repository::local_resource::LocalResourceRepository;
use shared::core_api::{NetStream, NetStreamEvent, NetStreamInner, UploadRequest, UploadResponse};
use shared::entities::file_system::file::LocalResourcePath;
use std::sync::Arc;
use futures::lock::Mutex;
use futures::{Stream, StreamExt};
use n0_future::SinkExt;
use n0_future::task::spawn;
use url::Url;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{ProgressEvent, XmlHttpRequest};

pub struct NetStreamImpl {
    pub storage: FileStorage,
    pub resource_repo: Arc<dyn LocalResourceRepository>,
}

pub struct NetStreamInnerImpl {
    storage: FileStorage,
    pub resource_repo: Arc<dyn LocalResourceRepository>,
    requests: Vec<UploadRequest>,
    path: LocalResourcePath,
}

impl NetStreamImpl {
    pub fn new(storage: FileStorage, resource_repo: Arc<dyn LocalResourceRepository>) -> Self {
        Self {
            storage,
            resource_repo,
        }
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
        }))
    }
}

#[async_trait::async_trait(?Send)]
impl NetStreamInner for NetStreamInnerImpl {
    async fn start(&mut self) -> anyhow::Result<UnboundedReceiver<NetStreamEvent>> {
        let (tx, rx) = mpsc::unbounded();
        
        if self.requests.is_empty() {
            let _ = tx.unbounded_send(NetStreamEvent::Completed(vec![]));
            return Ok(rx);
        }

        let LocalResourcePath::PlatformIdentifier(_) = &self.path else {
            return Err(anyhow::anyhow!("Invalid local resource path, expected platform identifier"));
        };

        // Clone necessary data for the spawned task
        let requests = self.requests.clone();
        let path = self.path.clone();
        let storage = self.storage.clone();
        let resource_repo = self.resource_repo.clone();

        // Spawn a task to handle all uploads sequentially
        spawn(async move {
            Self::upload_all_requests(requests, path, storage, resource_repo, tx).await;
        });
        
        Ok(rx)
    }

    async fn end(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl NetStreamInnerImpl {
    async fn upload_all_requests(
        requests: Vec<UploadRequest>,
        path: LocalResourcePath,
        storage: FileStorage,
        resource_repo: Arc<dyn LocalResourceRepository>,
        tx: mpsc::UnboundedSender<NetStreamEvent>,
    ) {
        let mut responses = Vec::new();
        let mut total_uploaded_bytes = 0u64;

        for (index, request) in requests.iter().enumerate() {
            match Self::upload_single_request(
                request,
                &path,
                &storage,
                &resource_repo,
                index,
                &requests,
                total_uploaded_bytes,
                tx.clone(),
            ).await {
                Ok(response) => {
                    responses.push(response);
                    total_uploaded_bytes += request.x_content_length;
                }
                Err(e) => {
                    let _ = tx.unbounded_send(NetStreamEvent::Error(e));
                    return;
                }
            }
        }

        let _ = tx.unbounded_send(NetStreamEvent::Completed(responses));
    }

    async fn upload_single_request(
        request: &UploadRequest,
        path: &LocalResourcePath,
        storage: &FileStorage,
        resource_repo: &Arc<dyn LocalResourceRepository>,
        request_index: usize,
        all_requests: &[UploadRequest],
        bytes_uploaded_so_far: u64,
        tx: mpsc::UnboundedSender<NetStreamEvent>,
    ) -> anyhow::Result<UploadResponse> {
        use n0_future::future::pending;
        use futures::future::select;
        use std::pin::Pin;

        let xhr = Arc::new(NeverSend(XmlHttpRequest::new().unwrap()));
        
        xhr.open_with_async("PUT", request.url.as_str(), true).unwrap();
        xhr.set_request_header("Content-Type", "application/octet-stream").unwrap();

        let (completion_tx, mut completion_rx) = mpsc::unbounded::<Result<UploadResponse, anyhow::Error>>();

        // Setup completion handler
        {
            let xhr_clone = xhr.clone();
            let mut completion_tx = completion_tx.clone();
            
            let onload_cb = Closure::<dyn FnMut()>::new(move || {
                let status_code = xhr_clone.status().unwrap_or(0);
                
                let result = if (200..300).contains(&status_code) {
                    let headers: HashMap<String, String> = xhr_clone
                        .get_all_response_headers()
                        .ok()
                        .map(|raw| {
                            raw.split("\r\n")
                                .filter_map(|line| {
                                    if let Some((key, value)) = line.split_once(':') {
                                        Some((key.trim().to_string(), value.trim().to_string()))
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let json = xhr_clone.response().ok()
                        .and_then(|js_value| serde_wasm_bindgen::from_value(js_value).ok());
                    Ok(UploadResponse { headers, json })
                } else {
                    let text_response = xhr_clone.response_text();
                    Err(anyhow!(
                        "Server response status {status_code} - {text_response:?}"
                    ))
                };

                let _ = completion_tx.unbounded_send(result);
            });

            xhr.set_onload(Some(onload_cb.as_ref().unchecked_ref()));
            onload_cb.forget();
        }

        // Setup progress handler
        {
            let mut tx_clone = tx.clone();
            
            let progress_cb = Closure::<dyn FnMut(_)>::new(move |event: ProgressEvent| {
                let loaded = event.loaded() as u64;
                let total_progress = bytes_uploaded_so_far + loaded;
                let _ = tx_clone.unbounded_send(NetStreamEvent::Progress {
                    uploaded_bytes: total_progress
                });
            });

            let x_upload = xhr.upload().unwrap();
            x_upload.set_onprogress(Some(progress_cb.as_ref().unchecked_ref()));
            progress_cb.forget();
        }

        // Setup error handler
        {
            let mut completion_tx = completion_tx.clone();
            let error_cb = Closure::<dyn FnMut()>::new(move || {
                let _ = completion_tx.unbounded_send(Err(anyhow!("Network level error")));
            });

            xhr.set_onerror(Some(error_cb.as_ref().unchecked_ref()));
            error_cb.forget();
        }

        // Setup timeout handler
        {
            let mut completion_tx = completion_tx.clone();
            let timeout_cb = Closure::<dyn FnMut()>::new(move || {
                let _ = completion_tx.unbounded_send(Err(anyhow!("Timeout")));
            });
            xhr.set_ontimeout(Some(timeout_cb.as_ref().unchecked_ref()));
            timeout_cb.forget();
        }

        // Send the data
        Self::send_request_data(&xhr, request, path, storage, resource_repo, request_index, all_requests).await?;
        
        // Wait for completion
        completion_rx.next().await.unwrap_or_else(|| Err(anyhow!("Upload completion channel closed")))
    }

    async fn send_request_data(
        xhr: &Arc<NeverSend<XmlHttpRequest>>,
        request: &UploadRequest,
        path: &LocalResourcePath,
        storage: &FileStorage,
        resource_repo: &Arc<dyn LocalResourceRepository>,
        request_index: usize,
        all_requests: &[UploadRequest],
    ) -> anyhow::Result<()> {
        let LocalResourcePath::PlatformIdentifier(_) = path else {
            return Err(anyhow!("Invalid local resource path"));
        };

        if let Some(device_file_id) = path.device_file_id() {
            let Some(device_file) = storage.get(device_file_id).await.map(|it| it.file) else {
                return Err(anyhow!("File not found"));
            };

            // Calculate the byte range for this request
            let mut bytes_to_skip = 0u64;
            for i in 0..request_index {
                bytes_to_skip += all_requests[i].x_content_length;
            }

            let from = bytes_to_skip as f64;
            let to = (bytes_to_skip + request.x_content_length).min(device_file.size() as u64) as f64;
            
            let blob = device_file.slice_with_f64_and_f64(from, to)
                .map_err(|e| anyhow!("Failed to slice file: {:?}", e))?;
            
            xhr.send_with_opt_blob(Some(&blob))
                .map_err(|e| anyhow!("Upload file error: {:?}", e))?;
        } else if let Ok(mut reader) = resource_repo.read(path.clone(), 1024 * 1024).await {
            // For small resources, read all at once and slice appropriately
            let all_bytes = reader.read_all().await?;
            
            let mut bytes_to_skip = 0usize;
            for i in 0..request_index {
                bytes_to_skip += all_requests[i].x_content_length as usize;
            }
            
            let start = bytes_to_skip.min(all_bytes.len());
            let end = (bytes_to_skip + request.x_content_length as usize).min(all_bytes.len());
            let slice = &all_bytes[start..end].to_vec().into_uint_array();
            
            xhr.send_with_opt_js_u8_array(Some(&slice))
                .map_err(|e| anyhow!("Upload error: {:?}", e))?;
        } else {
            return Err(anyhow!("Cannot read resource"));
        }

        Ok(())
    }
}
