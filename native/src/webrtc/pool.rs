use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use str0m::channel::ChannelId;

use crate::webrtc::client::WebRtcClientError;
use crate::webrtc::rtc::{ChannelIds, RtcEvent, RtcHandle};

pub struct Slot {
    pub index: usize,
    pub handle: RtcHandle,
}

impl Slot {
    pub fn is_alive(&self) -> bool {
        self.handle.is_alive()
    }

    pub fn data_buffered_amount(&self) -> usize {
        self.handle.data_buffered_amount()
    }

    pub fn is_relay(&self) -> bool {
        self.handle.is_relay()
    }

    pub fn send(&self, data: &[u8], channel_id: ChannelId) -> bool {
        self.handle.send(data, channel_id)
    }
}

pub struct ConnectionPool {
    slots: Box<[OnceLock<Slot>]>,
    channel_ids: ChannelIds,
    event_tx: tokio::sync::mpsc::Sender<(usize, RtcEvent)>,
    closed: AtomicBool,
}

impl ConnectionPool {
    pub fn new_with_primary(
        mut primary: RtcHandle,
        total_slots: usize,
        event_tx: tokio::sync::mpsc::Sender<(usize, RtcEvent)>,
    ) -> Arc<Self> {
        let channel_ids = *primary.channel_ids();
        let primary_event_rx = primary.take_event_rx();

        let n = total_slots.max(1);
        let slots: Box<[OnceLock<Slot>]> = (0..n).map(|_| OnceLock::new()).collect();

        slots[0]
            .set(Slot { index: 0, handle: primary })
            .ok()
            .expect("primary slot must start empty");

        let pool = Arc::new(Self {
            slots,
            channel_ids,
            event_tx: event_tx.clone(),
            closed: AtomicBool::new(false),
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
                    let installed = match pool.slots.get(idx) {
                        Some(cell) => cell.set(Slot { index: idx, handle }).is_ok(),
                        None => false,
                    };
                    if installed {
                        log::info!("[pool] slot {} connected and ready", idx);
                        if let Some(rx) = event_rx {
                            Self::spawn_slot_event_forwarder(pool, idx, rx);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("[pool] slot {} failed to connect: {:?}", idx, e);
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
        });
    }

    pub fn try_send_reliable(&self, data: &[u8]) -> bool {
        if self.closed.load(Ordering::Relaxed) {
            return false;
        }

        let candidates = self
            .slots
            .iter()
            .filter_map(|cell| cell.get())
            .filter(|s| s.is_alive())
            .map(|s| (s.index, s.data_buffered_amount()));

        let Some(idx) = pick_least_buffered(candidates) else {
            return false;
        };
        match self.slots.get(idx).and_then(|cell| cell.get()) {
            Some(slot) => slot.send(data, self.channel_ids.reliable),
            None => false,
        }
    }

    pub fn try_send_control(&self, data: &[u8], channel_id: ChannelId) -> bool {
        if self.closed.load(Ordering::Relaxed) {
            return false;
        }

        let Some(slot) = self.slots.first().and_then(|cell| cell.get()) else {
            return false;
        };
        if !slot.is_alive() {
            return false;
        }
        slot.send(data, channel_id)
    }

    pub fn slot0_alive(&self) -> bool {
        if self.closed.load(Ordering::Relaxed) {
            return false;
        }
        self.slots
            .first()
            .and_then(|cell| cell.get())
            .is_some_and(|s| s.is_alive())
    }

    pub fn alive_count(&self) -> usize {
        if self.closed.load(Ordering::Relaxed) {
            return 0;
        }
        self.slots
            .iter()
            .filter_map(|cell| cell.get())
            .filter(|s| s.is_alive())
            .count()
    }

    pub fn any_slot_is_relay(&self) -> bool {
        if self.closed.load(Ordering::Relaxed) {
            return false;
        }
        self.slots
            .iter()
            .filter_map(|cell| cell.get())
            .filter(|s| s.is_alive())
            .any(|s| s.is_relay())
    }

    pub fn shutdown_all(&self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}

fn pick_least_buffered<I>(candidates: I) -> Option<usize>
where
    I: IntoIterator<Item = (usize, usize)>,
{
    let mut best: Option<(usize, usize)> = None;
    for (idx, buffered) in candidates {
        match best {
            None => best = Some((idx, buffered)),
            Some((_, best_buf)) if buffered < best_buf => best = Some((idx, buffered)),
            _ => {}
        }
    }
    best.map(|(idx, _)| idx)
}

#[cfg(test)]
mod tests {
    use super::pick_least_buffered;

    #[test]
    fn pick_least_buffered_returns_none_when_empty() {
        let chosen = pick_least_buffered(std::iter::empty());
        assert!(chosen.is_none());
    }

    #[test]
    fn pick_least_buffered_returns_only_candidate() {
        let chosen = pick_least_buffered([(7usize, 1024usize)]);
        assert_eq!(chosen, Some(7));
    }

    #[test]
    fn pick_least_buffered_picks_smallest_buffer() {
        let chosen = pick_least_buffered([(0, 4096usize), (1, 512), (2, 2048)]);
        assert_eq!(chosen, Some(1));
    }

    #[test]
    fn pick_least_buffered_tie_prefers_first_seen() {
        let chosen = pick_least_buffered([(0, 1024usize), (1, 1024), (2, 1024)]);
        assert_eq!(chosen, Some(0));
    }

    #[test]
    fn pick_least_buffered_zero_buffer_wins() {
        let chosen = pick_least_buffered([(0, 500usize), (1, 0), (2, 250)]);
        assert_eq!(chosen, Some(1));
    }
}
