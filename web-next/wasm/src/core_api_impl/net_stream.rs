use std::sync::Arc;
use bytes::Bytes;
use n0_future::task::{JoinHandle, spawn};
use url::Url;
use futures::channel::mpsc;
use futures::{Stream, StreamExt};
use futures::lock::Mutex;
use js_sys::Uint8Array;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, Headers, ReadableStream, ReadableStreamDefaultController, Request, RequestInit};
use shared::core_api::{NetStream, NetStreamInner};
use shared::core_transfer_protocol::public_cloud::cloud_service::CloudTransferErrors;
use crate::errors::JsError;

pub struct NetStreamImpl {}

pub struct NetStreamInnerImpl {
    handle: Option<JoinHandle<Result<(), CloudTransferErrors>>>,
    writer: mpsc::Sender<bytes::Bytes>,
}

#[async_trait::async_trait(?Send)]
impl NetStream for NetStreamImpl {
    async fn start(&self, http_url: Url, size: u64) -> anyhow::Result<Box<dyn NetStreamInner>> {
        let (writer, mut reader) = mpsc::channel::<Bytes>(1024 * 512);
        let reader = Arc::new(Mutex::new(reader));
        let handle = spawn(async move {
            let reader = reader.clone();
            let stream_closure = Closure::wrap(Box::new(move |controller_val: JsValue| {
                // Clone immediately
                let controller_val_clone = controller_val.clone();
                let mut reader = reader.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    let controller = controller_val_clone
                        .dyn_into::<ReadableStreamDefaultController>()
                        .unwrap();

                    if let Some(chunk) = reader.lock().await.next().await {
                        let js_chunk = js_sys::Uint8Array::from(&chunk[..]);
                        let _ = controller.enqueue_with_chunk(&js_chunk);
                    } else {
                        if let Err(e) = controller.close() {
                            log::info!("Error while closing the upload stream: {e:?}");
                        }
                    }
                });
            }) as Box<dyn FnMut(JsValue)>);

            let underlying_source = js_sys::Object::new();
            js_sys::Reflect::set(
                &underlying_source,
                &JsValue::from_str("pull"),
                stream_closure.as_ref().unchecked_ref(),
            ).map_err(|it| JsError(it))?;
            let js_stream = ReadableStream::new_with_underlying_source(&underlying_source).map_err(|it| JsError(it))?;

            let js_value = js_stream.into();

            // Prepare fetch request
            let mut opts = RequestInit::new();
            opts.set_method("PUT");
            opts.set_body(&js_value);

            let headers = Headers::new().map_err(|it| JsError(it))?;
            headers.set("Content-Type", "application/octet-stream").map_err(|it| JsError(it))?;
            headers.set("Content-Length", &size.to_string()).map_err(|it| JsError(it))?;
            opts.set_headers(&headers);

            let req = Request::new_with_str_and_init(
                http_url.as_str(),
                &opts,
            ).map_err(|it| JsError(it))?;

            let resp_value = JsFuture::from(
                window().unwrap().fetch_with_request(&req)
            ).await.map_err(|it| JsError(it))?;

            let resp = resp_value.dyn_into::<web_sys::Response>().map_err(|it| JsError(it))?;
            log::info!("Upload completed {resp:?}");
            Ok(())
        });

        Ok(Box::new(NetStreamInnerImpl {
            writer,
            handle: Some(handle),
        }))
    }
}

#[async_trait::async_trait(?Send)]
impl NetStreamInner for NetStreamInnerImpl {
    async fn write(&mut self, data: Bytes) -> anyhow::Result<()> {
        self.writer.try_send(data)?;

        Ok(())
    }

    async fn end(&mut self) -> anyhow::Result<()> {
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };

        let Ok(result) = handle.await else { return Ok(()) };

        result?;

        Ok(())
    }
}

impl Drop for NetStreamInnerImpl {
    fn drop(&mut self) {
        let _ = self.end();
    }
}
