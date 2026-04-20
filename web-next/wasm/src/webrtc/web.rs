use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use futures::channel::mpsc;
use futures::stream::StreamExt;
use js_sys::{Array, ArrayBuffer, Uint8Array};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Event, MessageEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelInit, RtcDataChannelType, RtcIceServer,
    RtcIceTransportPolicy, RtcPeerConnection, RtcSdpType, RtcSessionDescriptionInit,
};

pub struct RtcConnectionWrapper(pub(crate) RtcPeerConnection);

unsafe impl Send for RtcConnectionWrapper {}
unsafe impl Sync for RtcConnectionWrapper {}

impl Deref for RtcConnectionWrapper {
    type Target = RtcPeerConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RtcConnectionWrapper {
    pub fn new(conn: RtcPeerConnection) -> Arc<Self> {
        Arc::new(Self(conn))
    }
}

impl Drop for RtcConnectionWrapper {
    fn drop(&mut self) {
        log::info!("closing peer connection on drop");
        self.0.close();
    }
}

pub struct RtcDataChannelWrapper(pub(crate) RtcDataChannel);

unsafe impl Send for RtcDataChannelWrapper {}
unsafe impl Sync for RtcDataChannelWrapper {}

impl Deref for RtcDataChannelWrapper {
    type Target = RtcDataChannel;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RtcDataChannelWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for RtcDataChannelWrapper {
    fn drop(&mut self) {
        log::info!("closing data channel on drop");
        self.0.close();
    }
}

#[derive(Clone, Debug)]
pub struct ChannelConfig {
    pub ordered: bool,
    pub max_retransmits: Option<u16>,
    pub buffer_low_threshold: Option<usize>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            ordered: true,
            max_retransmits: None,
            buffer_low_threshold: Some(16 * 1024),
        }
    }
}

pub mod channel_ids {
    pub const RELIABLE_DATA_CHANNEL_ID: u16 = 1;
    pub const UNORDERED_MSG_CHANNEL_ID: u16 = 2;
    pub const ORDERED_MSG_CHANNEL_ID: u16 = 3;
}

pub struct WebRtcApi {
    config: schema::devlog::rpc_signalling::server::IceConfig,
}

impl WebRtcApi {
    pub fn new(config: schema::devlog::rpc_signalling::server::IceConfig) -> Self {
        Self { config }
    }

    pub fn create_peer_connection(&self) -> Result<Arc<RtcConnectionWrapper>, WebError> {
        let config = RtcConfiguration::new();
        let ice_servers_array = Array::new();

        for url in &self.config.urls {
            let server = RtcIceServer::new();
            server.set_urls(&JsValue::from_str(url));
            if let Some(user) = &self.config.username {
                server.set_username(user);
            }
            if let Some(cred) = &self.config.credential {
                server.set_credential(cred);
            }
            ice_servers_array.push(&server);
        }

        config.set_ice_servers(&ice_servers_array);

        if crate::config::is_relay_only() {
            config.set_ice_transport_policy(RtcIceTransportPolicy::Relay);
        }

        let conn = RtcPeerConnection::new_with_configuration(&config).map_err(|e| WebError::Connection(format!("{:?}", e)))?;
        Ok(RtcConnectionWrapper::new(conn))
    }

    pub fn create_reliable_channel(
        &self,
        connection: Arc<RtcConnectionWrapper>,
        channel_id: u16,
    ) -> Result<Arc<RtcDataChannelWrapper>, WebError> {
        let config = RtcDataChannelInit::new();
        config.set_ordered(true);
        config.set_negotiated(true);
        config.set_id(channel_id);
        let channel = connection.create_data_channel_with_data_channel_dict("reliable", &config);
        channel.set_binary_type(RtcDataChannelType::Arraybuffer);
        Ok(Arc::new(RtcDataChannelWrapper(channel)))
    }

    pub fn create_unreliable_channel(
        &self,
        connection: Arc<RtcConnectionWrapper>,
        channel_id: u16,
    ) -> Result<Arc<RtcDataChannelWrapper>, WebError> {
        let config = RtcDataChannelInit::new();
        config.set_ordered(false);
        config.set_max_retransmits(0);
        config.set_negotiated(true);
        config.set_id(channel_id);
        let channel = connection.create_data_channel_with_data_channel_dict("unreliable", &config);
        channel.set_binary_type(RtcDataChannelType::Arraybuffer);
        Ok(Arc::new(RtcDataChannelWrapper(channel)))
    }

    pub fn create_unordered_channel(
        &self,
        connection: Arc<RtcConnectionWrapper>,
        channel_id: u16,
    ) -> Result<Arc<RtcDataChannelWrapper>, WebError> {
        let config = RtcDataChannelInit::new();
        config.set_ordered(false);
        config.set_negotiated(true);
        config.set_id(channel_id);
        let channel = connection.create_data_channel_with_data_channel_dict("unordered", &config);
        channel.set_binary_type(RtcDataChannelType::Arraybuffer);
        Ok(Arc::new(RtcDataChannelWrapper(channel)))
    }

    pub fn create_ordered_channel(
        &self,
        connection: Arc<RtcConnectionWrapper>,
        channel_id: u16,
    ) -> Result<Arc<RtcDataChannelWrapper>, WebError> {
        let config = RtcDataChannelInit::new();
        config.set_ordered(true);
        config.set_negotiated(true);
        config.set_id(channel_id);
        let channel = connection.create_data_channel_with_data_channel_dict("ordered-msg", &config);
        channel.set_binary_type(RtcDataChannelType::Arraybuffer);
        Ok(Arc::new(RtcDataChannelWrapper(channel)))
    }

    pub fn setup_channel_handlers(
        &self,
        channel: Arc<RtcDataChannelWrapper>,
        inbound_tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<(), WebError> {
        let onmessage = {
            let inbound_tx = inbound_tx.clone();
            let channel = channel.clone();
            Closure::wrap(Box::new(move |event: MessageEvent| {
                if let Ok(arraybuf) = event.data().dyn_into::<ArrayBuffer>() {
                    let packet: Vec<u8> = Uint8Array::new(&arraybuf).to_vec();
                    if let Err(e) = inbound_tx.unbounded_send(packet) {
                        log::warn!("Failed to send inbound packet: {:?}", e);
                    }
                } else {
                    log::warn!(
                        "WASM onmessage: data is not ArrayBuffer on channel {}",
                        channel.id().unwrap_or(0)
                    );
                }
            }) as Box<dyn FnMut(MessageEvent)>)
        };
        channel.clone().set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        let onerror = {
            let channel = channel.clone();
            Closure::wrap(Box::new(move |event: JsValue| {
                log::error!("DataChannel {} error: {:?}", channel.id().unwrap_or(0), event);
            }) as Box<dyn FnMut(JsValue)>)
        };
        channel.clone().set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        let onclose = {
            let channel = channel.clone();
            Closure::wrap(Box::new(move |_event: Event| {
                log::info!("Data channel {} closed", channel.id().unwrap_or(0));
            }) as Box<dyn FnMut(Event)>)
        };
        channel.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        onclose.forget();

        Ok(())
    }

    pub async fn create_offer_and_set_local(&self, connection: &RtcConnectionWrapper) -> Result<String, WebError> {
        let offer = JsFuture::from(connection.create_offer())
            .await
            .map_err(|e| WebError::Sdp(format!("create_offer failed: {:?}", e)))?;
        let sdp = js_sys::Reflect::get(&offer, &"sdp".into())
            .map_err(|e| WebError::Sdp(format!("Failed to get sdp: {:?}", e)))?
            .as_string()
            .ok_or_else(|| WebError::Sdp("SDP is not a string".to_string()))?;

        let offer_desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_desc.set_sdp(&sdp);
        JsFuture::from(connection.set_local_description(&offer_desc))
            .await
            .map_err(|e| WebError::Sdp(format!("set_local_description failed: {:?}", e)))?;

        Ok(sdp)
    }

    pub async fn set_remote_description(&self, connection: &RtcConnectionWrapper, answer_sdp: &str) -> Result<(), WebError> {
        let answer_desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_desc.set_sdp(answer_sdp);
        JsFuture::from(connection.set_remote_description(&answer_desc))
            .await
            .map_err(|e| WebError::Sdp(format!("set_remote_description failed: {:?}", e)))?;
        Ok(())
    }

    pub async fn wait_for_channel_open(&self, channel: Arc<RtcDataChannelWrapper>) -> Result<(), WebError> {
        let channel_id = channel.id().unwrap_or(0);
        if channel.ready_state() == web_sys::RtcDataChannelState::Open {
            log::info!("Data channel {} is already open", channel_id);
            return Ok(());
        }

        let (tx, mut rx) = mpsc::channel::<()>(1);
        let onopen = Closure::wrap(Box::new(move |_v: JsValue| {
            let _ = tx.clone().try_send(());
        }) as Box<dyn FnMut(JsValue)>);
        channel.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        rx.next().await;
        channel.set_onopen(None);
        log::info!("Data channel {} opened", channel_id);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WebError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("SDP error: {0}")]
    Sdp(String),
}
