static _CURRENT_VERSION: &str = "1.0.0";

use core_services::logger::setup;
use tokio::sync::Mutex;
use tokio::time::{self, Duration, Interval};
pub mod config;
mod core_api_impl;
pub mod di_container;
pub mod native;
pub mod network;
pub mod repository;
pub mod webrtc;

use crate::core_api_impl::bridge::CoreBridgeImpl;
use crate::native::message_to_shell::{MessageToShell, MessageToShellResponse};
use crate::repository::path_resolver::PathResolverImpl;
use bincode::Options;
pub use crux_core::bridge::Bridge;
pub use crux_core::{Core, Request};
use di_container::DiContainer;
use erased_serde::Serialize;
use lazy_static::lazy_static;
use native::executor::NativeExecutor;
use shared::app::operations::CoreOperation;
use shared::app::BitBridge;
use shared::shell::api::{CoreBridge, CoreRequest, CruxRequest};
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::OnceCell;
use tokio::task::JoinHandle;

pub static TOKIO_RT: OnceCell<tokio::runtime::Runtime> = OnceCell::const_new();

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

    async fn notify(self: Arc<Self>, msg: MessageToShell) -> MessageToShellResponse {
        let self_clone = self.clone();
        self_clone.request(msg).await
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

        let join_handle = get_tokio_rt().spawn(async move {
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

impl<E: Serialize + Send + 'static> Drop for ThrottleShellRuntime<E> {
    fn drop(&mut self) {
        let handle = self.join_handle.abort_handle();
        handle.abort();
    }
}

pub struct NativeProcessor {
    native_executor: &'static NativeExecutor
}

pub fn get_tokio_rt() -> &'static tokio::runtime::Runtime {
    TOKIO_RT.get().unwrap_or_else(|| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_name("bitbridge-executor-worker")
            .enable_all()
            .build()
            .unwrap();
        TOKIO_RT.set(rt).unwrap();
        TOKIO_RT.get().expect("Tokio runtime not initialized")
    })
}

impl NativeProcessor {
    pub async fn new(shell: Arc<dyn ShellRuntime>) -> Self {
        setup();
        let di_container = DiContainer::get_instance();

        let _ = get_tokio_rt()
            .spawn({
                let shell = shell.clone();
                async move {
                    let core: &'static dyn CoreBridge = Box::leak(Box::new(CoreBridgeImpl::new(shell.clone())));
                    di_container.init(Arc::new(PathResolverImpl { shell: shell.clone() }), core).await;
                }
            })
            .await;

        let native_executor = di_container.get_native_executor();

        Self { native_executor }
    }

    pub async fn handle(&self, id: u32, effect: Vec<u8>) -> Vec<u8> {
        let options = bincode_options();
        let mut deser = bincode::Deserializer::from_slice(&effect, options);
        let mut deserializer = <dyn erased_serde::Deserializer>::erase(&mut deser);

        let effect: CoreOperation = erased_serde::deserialize(&mut deserializer).expect("Failed to deserialize effect");

        let native_executor = self.native_executor;
        let core_bridge = DiContainer::get_instance().core_bridge();
        get_tokio_rt()
            .spawn(async move {
                let output = native_executor.handle(CoreRequest::new(CruxRequest::Id(id), core_bridge), effect).await;
                handle_response(id, serialize(&output))
            })
            .await
            .unwrap_or_default()
    }
}

uniffi::include_scaffolding!("shared");

lazy_static! {
    pub static ref CORE_BRIDGE: Bridge<BitBridge> = Bridge::new(Core::new());
}

pub fn process_event(msg: Vec<u8>) -> Vec<u8> {
    CORE_BRIDGE.process_event(&msg).unwrap_or_default()
}

pub fn handle_response(id: u32, res: Vec<u8>) -> Vec<u8> {
    CORE_BRIDGE.handle_response(id, &res).unwrap_or_default()
}

pub fn view() -> Vec<u8> {
    CORE_BRIDGE.view().unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use crate::webrtc::ice::stun_url_to_host_port;

    #[test]
    fn defaults_ipv4_stun_urls_to_default_port() {
        assert_eq!(
            stun_url_to_host_port("stun:198.51.100.10"),
            Some("198.51.100.10:3478".to_string())
        );
    }

    #[test]
    fn preserves_bracketed_ipv6_with_explicit_port() {
        assert_eq!(
            stun_url_to_host_port("stun:[2001:db8::7]:3478"),
            Some("[2001:db8::7]:3478".to_string())
        );
    }

    #[test]
    fn brackets_raw_ipv6_literals_with_default_port() {
        assert_eq!(
            stun_url_to_host_port("stun:2001:db8::7"),
            Some("[2001:db8::7]:3478".to_string())
        );
    }
}
