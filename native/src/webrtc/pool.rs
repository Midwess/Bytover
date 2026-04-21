use std::sync::Arc;

use str0m::channel::ChannelId;
use tokio::sync::Mutex;

use crate::webrtc::client::WebRtcClientError;
use crate::webrtc::rtc::{ChannelIds, RtcEvent, RtcHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotState {
    Pending,
    Ready,
    Failed,
    Dead,
}

pub struct Slot {
    pub index: usize,
    pub handle: Option<RtcHandle>,
    pub state: SlotState,
}

impl Slot {
    fn is_ready(&self) -> bool {
        matches!(self.state, SlotState::Ready) && self.handle.as_ref().is_some_and(|h| h.is_alive())
    }
}

pub struct ConnectionPool {
    slots: Mutex<Vec<Slot>>,
    channel_ids: ChannelIds,
    event_tx: tokio::sync::mpsc::Sender<(usize, RtcEvent)>,
}

impl ConnectionPool {
    pub fn new_with_primary(
        mut primary: RtcHandle,
        total_slots: usize,
        event_tx: tokio::sync::mpsc::Sender<(usize, RtcEvent)>,
    ) -> Arc<Self> {
        let channel_ids = *primary.channel_ids();
        let primary_event_rx = primary.take_event_rx();

        let mut slots = Vec::with_capacity(total_slots.max(1));
        slots.push(Slot {
            index: 0,
            handle: Some(primary),
            state: SlotState::Ready,
        });
        for idx in 1..total_slots.max(1) {
            slots.push(Slot {
                index: idx,
                handle: None,
                state: SlotState::Pending,
            });
        }

        let pool = Arc::new(Self {
            slots: Mutex::new(slots),
            channel_ids,
            event_tx: event_tx.clone(),
        });

        if let Some(rx) = primary_event_rx {
            Self::spawn_slot_event_forwarder(Arc::clone(&pool), 0, rx);
        }

        pool
    }

    pub fn channel_ids(&self) -> ChannelIds {
        self.channel_ids
    }

    pub fn spawn_lazy_slot<F>(self: &Arc<Self>, idx: usize, connect_fut: F)
    where
        F: std::future::Future<Output = Result<RtcHandle, WebRtcClientError>> + Send + 'static,
    {
        let pool = Arc::clone(self);
        tokio::spawn(async move {
            match connect_fut.await {
                Ok(mut handle) => {
                    let event_rx = handle.take_event_rx();
                    let installed = {
                        let mut slots = pool.slots.lock().await;
                        if let Some(slot) = slots.get_mut(idx) {
                            slot.handle = Some(handle);
                            slot.state = SlotState::Ready;
                            true
                        } else {
                            false
                        }
                    };
                    if installed {
                        log::info!("[pool] slot {} connected and ready", idx);
                        if let Some(rx) = event_rx {
                            Self::spawn_slot_event_forwarder(Arc::clone(&pool), idx, rx);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("[pool] slot {} failed to connect: {:?}", idx, e);
                    let mut slots = pool.slots.lock().await;
                    if let Some(slot) = slots.get_mut(idx) {
                        slot.state = SlotState::Failed;
                    }
                }
            }
        });
    }

    fn spawn_slot_event_forwarder(pool: Arc<Self>, idx: usize, mut event_rx: tokio::sync::mpsc::Receiver<RtcEvent>) {
        let event_tx = pool.event_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                if event_tx.send((idx, event)).await.is_err() {
                    return;
                }
            }

            let mut slots = pool.slots.lock().await;
            if let Some(slot) = slots.get_mut(idx) {
                slot.state = SlotState::Dead;
                slot.handle = None;
            }
        });
    }

    pub async fn try_send_reliable(&self, data: &[u8]) -> bool {
        let slots = self.slots.lock().await;
        let mut best: Option<(usize, usize)> = None;
        for slot in slots.iter() {
            if !slot.is_ready() {
                continue;
            }
            let handle = match slot.handle.as_ref() {
                Some(h) => h,
                None => continue,
            };
            let buffered = handle.buffered_amount();
            match best {
                None => best = Some((slot.index, buffered)),
                Some((_, best_buf)) if buffered < best_buf => best = Some((slot.index, buffered)),
                _ => {}
            }
        }

        let Some((idx, _)) = best else { return false };
        match slots.get(idx).and_then(|s| s.handle.as_ref()) {
            Some(handle) => handle.send(data, self.channel_ids.reliable),
            None => false,
        }
    }

    pub async fn try_send_control(&self, data: &[u8], channel_id: ChannelId) -> bool {
        let slots = self.slots.lock().await;
        let Some(slot) = slots.first() else { return false };
        if !slot.is_ready() {
            return false;
        }
        match slot.handle.as_ref() {
            Some(handle) => handle.send(data, channel_id),
            None => false,
        }
    }

    pub async fn slot0_alive(&self) -> bool {
        let slots = self.slots.lock().await;
        slots.first().map(|s| s.is_ready()).unwrap_or(false)
    }

    pub async fn alive_count(&self) -> usize {
        let slots = self.slots.lock().await;
        slots.iter().filter(|s| s.is_ready()).count()
    }

    pub async fn shutdown_all(&self) {
        let mut slots = self.slots.lock().await;
        for slot in slots.iter_mut() {
            if let Some(mut handle) = slot.handle.take() {
                handle.shutdown();
            }
            slot.state = SlotState::Dead;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Slot, SlotState};

    #[test]
    fn slot_without_handle_is_not_ready() {
        let slot = Slot {
            index: 0,
            handle: None,
            state: SlotState::Ready,
        };
        assert!(!slot.is_ready());
    }

    #[test]
    fn slot_state_transitions() {
        assert_ne!(SlotState::Pending, SlotState::Ready);
        assert_ne!(SlotState::Ready, SlotState::Failed);
        assert_ne!(SlotState::Ready, SlotState::Dead);
    }
}
