use std::cell::RefCell;
use std::rc::Rc;

use futures::channel::mpsc;
use futures::stream::StreamExt;
use futures_timer::Delay;
use futures_util::{select_biased, FutureExt};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{RtcIceGatheringState, RtcPeerConnection};

#[derive(Default)]
struct IceCandidateTracker {
    host_candidates: usize,
    srflx_candidates: usize,
    relay_candidates: usize,
    prflx_candidates: usize,
}

impl IceCandidateTracker {
    fn add_candidate(&mut self, candidate: &str) {
        if candidate.contains("typ host") {
            self.host_candidates += 1;
        } else if candidate.contains("typ srflx") {
            self.srflx_candidates += 1;
        } else if candidate.contains("typ prflx") {
            self.prflx_candidates += 1;
        } else if candidate.contains("typ relay") {
            self.relay_candidates += 1;
        }
    }

    fn has_sufficient_candidates(&self) -> bool {
        self.host_candidates > 0 || self.srflx_candidates > 0 || self.relay_candidates > 0
    }
}

pub struct IceAgent {
    timeout_ms: u64,
    early_check_ms: u64,
    cap_ms: u64,
}

impl Default for IceAgent {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            early_check_ms: 1_000,
            cap_ms: 5_500,
        }
    }
}

impl IceAgent {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn wait_for_gathering_complete(&self, conn: &RtcPeerConnection) -> Result<(), IceError> {
        if conn.ice_gathering_state() == RtcIceGatheringState::Complete {
            return Ok(());
        }

        let (complete_tx, mut complete_rx) = mpsc::channel::<bool>(1);
        let tracker = Rc::new(RefCell::new(IceCandidateTracker::default()));

        let state_conn = conn.clone();
        let complete_tx_clone = complete_tx.clone();
        let onstatechange = Closure::wrap(Box::new(move || {
            if state_conn.ice_gathering_state() == RtcIceGatheringState::Complete {
                let _ = complete_tx_clone.clone().try_send(true);
            }
        }) as Box<dyn FnMut()>);
        conn.set_onicegatheringstatechange(Some(onstatechange.as_ref().unchecked_ref()));
        onstatechange.forget();

        let tracker_clone = tracker.clone();
        let oncandidate = Closure::wrap(Box::new(move |event: JsValue| {
            let candidate = js_sys::Reflect::get(&event, &"candidate".into());
            if let Ok(cand) = candidate {
                if !cand.is_null() {
                    if let Ok(sdp) = js_sys::Reflect::get(&cand, &"candidate".into()) {
                        if let Some(sdp_str) = sdp.as_string() {
                            tracker_clone.borrow_mut().add_candidate(&sdp_str);
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(JsValue)>);
        conn.set_onicecandidate(Some(oncandidate.as_ref().unchecked_ref()));
        oncandidate.forget();

        let timeout = std::time::Duration::from_millis(self.timeout_ms.min(self.cap_ms));
        let early_check = std::time::Duration::from_millis(self.early_check_ms);
        let timed_out = select_biased! {
            _ = complete_rx.next().fuse() => false,
            _ = Delay::new(timeout).fuse() => true,
            _ = Delay::new(early_check).fuse() => {
                let ready = tracker.borrow().has_sufficient_candidates();
                if ready {
                    false
                } else {
                    let remaining = timeout.saturating_sub(early_check);
                    select_biased! {
                        _ = complete_rx.next().fuse() => false,
                        _ = Delay::new(remaining).fuse() => true,
                    }
                }
            }
        };

        if timed_out {
            log::warn!("ICE gathering timed out after {}ms", self.timeout_ms);
        }

        conn.set_onicegatheringstatechange(None);
        conn.set_onicecandidate(None);

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IceError {
    #[error("Timeout")]
    Timeout,
}
