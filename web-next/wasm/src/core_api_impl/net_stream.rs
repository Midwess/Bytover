use std::rc::Rc;
use std::sync::Arc;
use anyhow::anyhow;
use bytes::Bytes;
use n0_future::task::{JoinHandle, spawn};
use url::Url;
use futures::channel::mpsc;
use futures::{Stream, StreamExt};
use futures::lock::Mutex;
use futures_channel::mpsc::UnboundedReceiver;
use js_sys::{Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, Headers, ProgressEvent, ReadableStream, ReadableStreamDefaultController, Request, RequestInit, RequestMode, XmlHttpRequest};
use core_services::utils::never_send::NeverSend;
use shared::app::file_system::file::LocalResourcePath;
use shared::core_api::{NetStream, NetStreamEvent, NetStreamInner};
use shared::core_transfer_protocol::public_cloud::cloud_service::CloudTransferErrors;
use crate::errors::JsError;
use crate::file_api::storage::FileStorage;

pub struct NetStreamImpl {
    pub storage: FileStorage,
}

pub struct NetStreamInnerImpl {
    storage: FileStorage,
    url: Url,
    size: u64,
    path: LocalResourcePath,
    xhr: Option<Arc<NeverSend<XmlHttpRequest>>>,
}

#[async_trait::async_trait(?Send)]
impl NetStream for NetStreamImpl {
    async fn upload_resource(&self, http_url: Url, path: LocalResourcePath, size: u64) -> anyhow::Result<Box<dyn NetStreamInner>> {
        Ok(Box::new(NetStreamInnerImpl {
            storage: self.storage.clone(),
            url: http_url,
            size,
            path,
            xhr: None,
        }))
    }
}

#[async_trait::async_trait(?Send)]
impl NetStreamInner for NetStreamInnerImpl {
    async fn start(&mut self) -> anyhow::Result<UnboundedReceiver<NetStreamEvent>> {
        let (tx, rx) = mpsc::unbounded();
        let LocalResourcePath::PlatformIdentifier(platform_identifier) = &self.path else {
            return Err(anyhow::anyhow!("Invalid local resource path, expected platform identifier"));
        };

        let xhr = Arc::new(NeverSend(XmlHttpRequest::new().unwrap()));

        xhr.open_with_async("PUT", self.url.as_str(), true).unwrap();
        xhr.set_request_header("Content-Type", "application/octet-stream").unwrap();

        {
            let xhr_clone = xhr.clone();
            let tx = tx.clone();
            let onload_cb = Closure::<dyn FnMut()>::new(move || {
                let status = xhr_clone.status().unwrap_or(0);
                log::info!("The upload process is completed with status {status}");
                if status >= 200 && status < 300 {
                    let _ = tx.unbounded_send(NetStreamEvent::Completed);
                } else {
                    let text_response = xhr_clone.response_text();
                    let _ = tx.unbounded_send(NetStreamEvent::Error(anyhow!("Server response status {status} - {text_response:?}")));
                }
            });

            xhr.set_onload(Some(onload_cb.as_ref().unchecked_ref()));
            onload_cb.forget();
        }

        {
            let tx = tx.clone();
            let progress_cb = Closure::<dyn FnMut(_)>::new(move |event: ProgressEvent| {
                let loaded = event.loaded();
                let _ = tx.unbounded_send(NetStreamEvent::Progress {
                    uploaded_bytes: loaded as u64
                });
            });

            let x_upload = xhr.upload().unwrap();
            x_upload.set_onprogress(Some(progress_cb.as_ref().unchecked_ref()));
            progress_cb.forget();
        }

        // ===== ERROR =====
        {
            let tx = tx.clone();
            let error_cb = Closure::<dyn FnMut()>::new(move || {
                let _ = tx.unbounded_send(NetStreamEvent::Error(anyhow!("Network level errors, cannot know exactly")));
            });

            xhr.set_onerror(Some(error_cb.as_ref().unchecked_ref()));
            error_cb.forget();
        }

        // ===== TIMEOUT =====
        {
            let tx = tx.clone();
            let timeout_cb = Closure::<dyn FnMut()>::new(move || {
                let _ = tx.unbounded_send(NetStreamEvent::Error(anyhow!("Timeout")));
            });
            xhr.set_ontimeout(Some(timeout_cb.as_ref().unchecked_ref()));
            timeout_cb.forget();
        }

        // ===== ABORT =====
        {
            let tx = tx.clone();
            let abort_cb = Closure::<dyn FnMut()>::new(move || {
                log::info!("The upload process is aborted");
                let _ = tx.unbounded_send(NetStreamEvent::Completed);
            });
            xhr.set_onabort(Some(abort_cb.as_ref().unchecked_ref()));
            abort_cb.forget();
        }

        let is_thumbnail = platform_identifier.starts_with("idb://thumbnail");

        if is_thumbnail {
            let Ok(thumbnail_resource_id) = platform_identifier.trim_start().trim_start_matches("idb://thumbnail/").parse::<u64>() else {
                return Err(anyhow::anyhow!("Invalid thumbnail resource id"));
            };

            let Some(bytes) = self.storage.read_thumbnail_bytes(thumbnail_resource_id).await else {
                return Err(anyhow!("Thumbnail not found for resource {thumbnail_resource_id}"));
            };

            let xhr = xhr.clone();
            xhr.send_with_opt_js_u8_array(Some(&bytes))
                .map_err(|it| anyhow!("Upload thumbnail errors {it:?}"))?;
        }
        else {
            let Some(file) = self.storage.get_file_by_path(&LocalResourcePath::PlatformIdentifier(platform_identifier.clone())).await else {
                return Err(anyhow!("Not found any file located at {platform_identifier:?}"))
            };

            let xhr = xhr.clone();
            xhr.send_with_opt_blob(Some(&file))
                .map_err(|it| anyhow!("Upload file errors {it:?}"))?;
        }

        self.xhr = Some(xhr);

        Ok(rx)
    }

    async fn end(&mut self) -> anyhow::Result<()> {
        if let Some(xhr) = self.xhr.take() {
            log::info!("Aborting upload");
            xhr.abort().map_err(|it| anyhow!("Abort upload errors {it:?}"))?;
        }

        Ok(())
    }
}

impl Drop for NetStreamInnerImpl {
    fn drop(&mut self) {
        if let Some(xhr) = self.xhr.take() {
            let _ = xhr.abort();
        }
    }
}
