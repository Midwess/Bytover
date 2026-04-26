pub const COMPRESSION_BLOCK_SIZE: usize = 128 * 1024;
pub const WIRE_PART_SIZE: usize = 16 * 1024;
pub const WEBRTC_PACKET_HEADER_LEN: usize = 12;

pub struct WebRtcPacket;

impl WebRtcPacket {
    #[inline]
    pub fn serialize(prefix: u16, offset: u64, part_index: u8, part_count: u8, payload: &[u8]) -> Vec<u8> {
        let len = payload.len();
        let mut vec = Vec::with_capacity(len + WEBRTC_PACKET_HEADER_LEN);
        unsafe {
            vec.set_len(len + WEBRTC_PACKET_HEADER_LEN);
            let ptr = vec.as_mut_ptr();
            std::ptr::copy_nonoverlapping(prefix.to_le_bytes().as_ptr(), ptr, 2);
            std::ptr::copy_nonoverlapping(offset.to_le_bytes().as_ptr(), ptr.add(2), 8);
            *ptr.add(10) = part_index;
            *ptr.add(11) = part_count;
            std::ptr::copy_nonoverlapping(payload.as_ptr(), ptr.add(WEBRTC_PACKET_HEADER_LEN), len);
        }
        vec
    }

    /// Reads the 12-byte header without touching the payload.
    ///
    /// Intentionally does NOT split the payload off into a separate buffer:
    /// the previous `deserialize` API did an in-place `ptr::copy` to shift the
    /// payload left by `WEBRTC_PACKET_HEADER_LEN` bytes, which added ~8 KB of
    /// memcpy per wire packet (128 KB per compression block on the receiver
    /// hot path). Callers instead slice `&data[WEBRTC_PACKET_HEADER_LEN..]`
    /// when they need the payload, keeping the copy count at one.
    #[inline]
    pub fn parse_header(data: &[u8]) -> Option<(u16, u64, u8, u8)> {
        if data.len() < WEBRTC_PACKET_HEADER_LEN {
            return None;
        }
        unsafe {
            let prefix = u16::from_le_bytes(*(data.as_ptr() as *const [u8; 2]));
            let offset = u64::from_le_bytes(*(data.as_ptr().add(2) as *const [u8; 8]));
            let part_index = *data.as_ptr().add(10);
            let part_count = *data.as_ptr().add(11);
            Some((prefix, offset, part_index, part_count))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_parse_header() {
        let prefix = 0x1234u16;
        let offset = 0x1122334455667788u64;
        let part_index = 7u8;
        let part_count = 16u8;
        let payload = vec![1, 2, 3, 4, 5];
        let serialized = WebRtcPacket::serialize(prefix, offset, part_index, part_count, &payload);
        assert_eq!(serialized.len(), WEBRTC_PACKET_HEADER_LEN + payload.len());
        assert_eq!(serialized[0], 0x34);
        assert_eq!(serialized[1], 0x12);
        assert_eq!(serialized[2], 0x88);
        assert_eq!(serialized[9], 0x11);
        assert_eq!(serialized[10], part_index);
        assert_eq!(serialized[11], part_count);
        assert_eq!(&serialized[WEBRTC_PACKET_HEADER_LEN..], &payload[..]);

        let (d_prefix, d_offset, d_part_index, d_part_count) =
            WebRtcPacket::parse_header(&serialized).expect("header should parse");
        assert_eq!(d_prefix, prefix);
        assert_eq!(d_offset, offset);
        assert_eq!(d_part_index, part_index);
        assert_eq!(d_part_count, part_count);
        assert_eq!(&serialized[WEBRTC_PACKET_HEADER_LEN..], &payload[..]);
    }

    #[test]
    fn test_empty_payload() {
        let prefix = 0xFFFFu16;
        let offset = 0u64;
        let payload: Vec<u8> = vec![];
        let serialized = WebRtcPacket::serialize(prefix, offset, 0, 1, &payload);
        assert_eq!(serialized.len(), WEBRTC_PACKET_HEADER_LEN);

        let (d_prefix, d_offset, d_part_index, d_part_count) =
            WebRtcPacket::parse_header(&serialized).expect("header should parse");
        assert_eq!(d_prefix, prefix);
        assert_eq!(d_offset, offset);
        assert_eq!(d_part_index, 0);
        assert_eq!(d_part_count, 1);
        assert!(serialized[WEBRTC_PACKET_HEADER_LEN..].is_empty());
    }

    #[test]
    fn test_single_part_delimiter() {
        let prefix = 42u16;
        let offset = 0u64;
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let serialized = WebRtcPacket::serialize(prefix, offset, 0, 1, &payload);
        let (_, _, part_index, part_count) = WebRtcPacket::parse_header(&serialized).expect("header should parse");
        assert_eq!(part_index, 0);
        assert_eq!(part_count, 1);
        assert_eq!(&serialized[WEBRTC_PACKET_HEADER_LEN..], &payload[..]);
    }

    #[test]
    fn test_last_part_of_sixteen() {
        let prefix = 1u16;
        let offset = 128u64 * 1024;
        let payload = vec![0xAA; 3000];
        let serialized = WebRtcPacket::serialize(prefix, offset, 15, 16, &payload);
        let (_, d_offset, part_index, part_count) = WebRtcPacket::parse_header(&serialized).expect("header should parse");
        assert_eq!(d_offset, offset);
        assert_eq!(part_index, 15);
        assert_eq!(part_count, 16);
        assert_eq!(&serialized[WEBRTC_PACKET_HEADER_LEN..], &payload[..]);
    }

    #[test]
    fn test_parse_header_rejects_short_input() {
        let short: [u8; 5] = [0, 0, 0, 0, 0];
        assert!(WebRtcPacket::parse_header(&short).is_none());
    }
}
