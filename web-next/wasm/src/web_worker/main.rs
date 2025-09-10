use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use futures::lock::Mutex;
use gloo_worker::{Spawnable, Worker, WorkerBridge};
use n0_future::task::spawn;
use serde::{Deserialize, Serialize};
use futures::channel::{oneshot, mpsc};
use futures::StreamExt;
use serde::de::DeserializeOwned;
use uuid::Uuid;
use crate::web_worker::codec::WorkerMessageCodec;

/// To control worker on main threads
#[derive(Serialize, Deserialize)]
pub struct WorkerMessage<R>
where
    R: Serialize,
{
    id: String,
    pub(crate) message: R,
}

impl<R> Deref for WorkerMessage<R>
where
    R: Serialize,
{
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.message
    }
}

unsafe impl<R> Send for WorkerMessage<R> where R: Serialize {}
unsafe impl<R> Sync for WorkerMessage<R> where R: Serialize {}

impl<R> WorkerMessage<R> where R: Serialize {
    pub fn new(message: R) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message,
        }
    }

    pub fn response(id: String, message: R) -> Self {
        Self {
            id,
            message
        }
    }
}

pub trait TrustedWorkerMessage: Serialize + DeserializeOwned + Send + Sync + 'static {
    fn id(&self) -> &str;
    fn set_id(&mut self, request_id: String);
}

impl<R> TrustedWorkerMessage for WorkerMessage<R> where R: Serialize + DeserializeOwned + Send + Sync + 'static {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_id(&mut self, request_id: String) {
        self.id = request_id;
    }
}

pub struct WebWorkerBridge<W: Worker>
where
    W::Input: TrustedWorkerMessage,
    W::Output: TrustedWorkerMessage,
    W: 'static
{
    bridge: WorkerBridge<W>,
    streams: Arc<Mutex<HashMap<String, oneshot::Sender<W::Output>>>>,
}

impl<W: Worker> WebWorkerBridge<W>
where
    W::Input: TrustedWorkerMessage,
    W::Output: TrustedWorkerMessage,
    W: 'static
{
    pub fn spawn(name: &str) -> WebWorkerBridge<W> {
        let (callback_call, mut callback) = mpsc::channel::<W::Output>(1024);
        let bridge = W::spawner()
            .encoding::<WorkerMessageCodec>()
            .callback(move |o| {
                 if let Err(e) = callback_call.clone().try_send(o) {
                     log::warn!("Failed to send message to main thread: {:?}", e);
                 }
            })
            .spawn(&format!("/{name}/worker.js"));

        let response_streams = Arc::new(Mutex::new(HashMap::<String, oneshot::Sender<W::Output>>::new()));
        spawn({
            let response_streams = response_streams.clone();
            async move {
                while let Some(msg) = callback.next().await {
                    let Some(response_stream) = response_streams.lock().await.remove(msg.id()) else {
                        continue;
                    };

                    if let Err(e) = response_stream.send(msg) {
                        log::warn!("Failed to send message to main thread");
                        continue;
                    }
                }
            }
        });

        Self {
            bridge,
            streams: response_streams,
        }
    }

    pub async fn send(&self, msg: W::Input) -> Option<W::Output> {
        let (sender, receiver) = oneshot::channel();
        self.streams.lock().await.insert(msg.id().to_string(), sender);
        self.bridge.send(msg);
        receiver.await.ok()
    }
}
