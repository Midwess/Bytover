pub mod network;
pub mod message_to_shell;
pub mod repository;
pub mod core_api_impl;

// /shared/src/lib.rs
use std::sync::{Arc, LazyLock};
use n0_future::task::{spawn, JoinHandle};
use std::time::Duration;
use bincode::Options;
pub use crux_core::{bridge::Bridge, Core, Request};
use erased_serde::{Serialize};
use futures::lock::Mutex;
use n0_future::time;
use n0_future::time::Interval;
use shared::app::BitBridge;
use crate::message_to_shell::{MessageToShell, MessageToShellResponse};

static CORE: LazyLock<Bridge<BitBridge>> = LazyLock::new(|| Bridge::new(Core::new()));

#[async_trait::async_trait]
pub trait ShellRuntime: Send + Sync + 'static {
    async fn msg_from_native(&self, event: Vec<u8>) -> Vec<u8>;
    fn msg_from_native_bg(self: Arc<Self>, event: Vec<u8>) -> JoinHandle<Vec<u8>> {
        let self_clone = self.clone();
        spawn(async move { self_clone.msg_from_native(event).await })
    }

    async fn request(&self, event: MessageToShell) -> MessageToShellResponse {
        let data = serialize(&event);
        let response_data = self.msg_from_native(data).await;
        let response: MessageToShellResponse = bincode::deserialize(&response_data).unwrap();
        response
    }

    fn notify(self: Arc<Self>, msg: MessageToShell) -> JoinHandle<MessageToShellResponse> {
        let self_clone = self.clone();
        spawn(async move { self_clone.request(msg).await })
    }
}

pub struct ThrottleShellRuntime<E: Serialize + Send + 'static> {
    latest_event: Arc<Mutex<Option<E>>>,
    join_handle: JoinHandle<()>
}

impl<E: Serialize + Send + Sync + 'static> ThrottleShellRuntime<E> {
    pub fn new(shell_runtime: Arc<dyn ShellRuntime>, delay: Duration) -> Self {
        let latest_event = Arc::new(Mutex::new(None::<E>));
        let latest_event_clone = latest_event.clone();
        let shell_runtime_clone = shell_runtime.clone();

        let join_handle = spawn(async move {
            let mut interval: Interval = time::interval(delay);
            interval.tick().await;

            loop {
                interval.tick().await;

                let event_to_send = {
                    let mut latest = latest_event_clone.lock().await;
                    latest.take()
                };

                if let Some(event) = event_to_send {
                    let serialized_event = serialize(&event);
                    shell_runtime_clone.clone().msg_from_native_bg(serialized_event);
                }
            }
        });

        Self { latest_event, join_handle }
    }

    pub async fn send(&self, event: E) {
        let mut latest = self.latest_event.lock().await;
        *latest = Some(event);
    }
}

/// Ask the core to process an event
/// # Panics
/// If the core fails to process the event
#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub fn process_event(data: &[u8]) -> Vec<u8> {
    match CORE.process_event(data) {
        Ok(effects) => effects,
        Err(e) => panic!("{e}"),
    }
}

/// Ask the core to handle a response
/// # Panics
/// If the core fails to handle the response
#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
    match CORE.handle_response(id, data) {
        Ok(effects) => effects,
        Err(e) => panic!("{e}"),
    }
}

#[wasm_bindgen::prelude::wasm_bindgen]
#[must_use]
pub fn view() -> Vec<u8> {
    match CORE.view() {
        Ok(view) => view,
        Err(e) => panic!("{e}"),
    }
}

pub fn serialize<E: Serialize>(data: &E) -> Vec<u8> {
    let options = bincode_options();
    let mut buffer = Vec::new();
    let mut serializer = bincode::Serializer::new(&mut buffer, options);
    erased_serde::serialize(data, &mut serializer).unwrap();
    buffer
}

fn bincode_options() -> impl bincode::Options + Copy {
    bincode::DefaultOptions::new().with_fixint_encoding().allow_trailing_bytes()
}
