//! TURN relay integration helpers for RtcClient.
//!
//! Provides types and utilities for integrating `turn_client_proto::TurnClientUdp`
//! as a co-resident sans-I/O driver alongside str0m in the RTC event loop.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::Instant;

use turn_client_proto::udp::TurnClientUdp;

/// Carries the fully-allocated TURN client and its routing metadata
/// out of ICE gathering and into the RtcClient event loop.
#[derive(Debug)]
pub struct TurnRelayInfo {
    /// The TURN state machine after a successful Allocate handshake.
    pub client: TurnClientUdp,
    /// Address of the TURN server (used for packet demux on receive).
    pub server_addr: SocketAddr,
    /// The relayed address allocated on the TURN server (used as the
    /// `destination` when reconstructing Receive for str0m, and as the
    /// source-match for output routing).
    pub relay_addr: SocketAddr,
    /// The wall-clock reference point established at allocation time.
    /// All `stun_proto::Instant` values are derived from this.
    pub stun_base: Instant,
    /// Cached next deadline for the TURN client, as a `std::time::Instant`.
    /// Updated by `drive_turn()` in the RTC event loop.
    pub cached_timeout: Instant,
    /// Peer addresses for which ChannelBind succeeded on this allocation.
    /// Populated from `TurnEvent::ChannelCreated`. The RTC loop gates
    /// deferred remote relay candidates on membership in this set.
    bound_channels: HashSet<SocketAddr>,
}

impl TurnRelayInfo {
    /// Create a new `TurnRelayInfo`.
    pub fn new(client: TurnClientUdp, server_addr: SocketAddr, relay_addr: SocketAddr, stun_base: Instant) -> Self {
        Self {
            client,
            server_addr,
            relay_addr,
            stun_base,
            cached_timeout: Instant::now(),
            bound_channels: HashSet::new(),
        }
    }

    pub fn have_bound_channel(&self, peer: SocketAddr) -> bool {
        self.bound_channels.contains(&peer)
    }

    pub fn mark_channel_bound(&mut self, peer: SocketAddr) {
        self.bound_channels.insert(peer);
    }

    pub fn mark_channel_unbound(&mut self, peer: SocketAddr) {
        self.bound_channels.remove(&peer);
    }
}

/// Convert `std::time::Instant` to `stun_proto::Instant`.
///
/// The `sans_io_time::Instant` (which `stun_proto::Instant` wraps) stores an
/// initial reference point and computes elapsed duration on each call to
/// `from_std`. Passing the same `base` gives an instant that reflects the
/// current time relative to that base.
#[inline]
pub fn stun_now(base: Instant) -> stun_proto::Instant {
    stun_proto::Instant::from_std(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stun_now_returns_different_values() {
        let base = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let now1 = stun_now(base);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let now2 = stun_now(base);
        // The two instants should represent different elapsed times
        assert!(now2 > now1, "stun_now should advance with time");
    }

    #[test]
    fn test_stun_now_from_same_base_is_monotonic() {
        let base = Instant::now();
        let now1 = stun_now(base);
        let now2 = stun_now(base);
        let now3 = stun_now(base);
        assert!(now3 >= now2);
        assert!(now2 >= now1);
    }
}
