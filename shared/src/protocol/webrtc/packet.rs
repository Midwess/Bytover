pub const COMPRESSION_BLOCK_SIZE: usize = 128 * 1024;
pub const WIRE_PART_SIZE: usize = 8 * 1024;
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

    #[inline]
    pub fn deserialize(mut data: Vec<u8>) -> (u16, u64, u8, u8, Vec<u8>) {
        if data.len() < WEBRTC_PACKET_HEADER_LEN {
            return (0, 0, 0, 0, data);
        }
        unsafe {
            let prefix = u16::from_le_bytes(*(data.as_ptr() as *const [u8; 2]));
            let offset = u64::from_le_bytes(*(data.as_ptr().add(2) as *const [u8; 8]));
            let part_index = *data.as_ptr().add(10);
            let part_count = *data.as_ptr().add(11);

            let len = data.len();
            std::ptr::copy(data.as_ptr().add(WEBRTC_PACKET_HEADER_LEN), data.as_mut_ptr(), len - WEBRTC_PACKET_HEADER_LEN);
            data.set_len(len - WEBRTC_PACKET_HEADER_LEN);

            (prefix, offset, part_index, part_count, data)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize() {
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

        let (d_prefix, d_offset, d_part_index, d_part_count, d_payload) = WebRtcPacket::deserialize(serialized);
        assert_eq!(d_prefix, prefix);
        assert_eq!(d_offset, offset);
        assert_eq!(d_part_index, part_index);
        assert_eq!(d_part_count, part_count);
        assert_eq!(d_payload, payload);
    }

    #[test]
    fn test_empty_payload() {
        let prefix = 0xFFFFu16;
        let offset = 0u64;
        let payload = vec![];
        let serialized = WebRtcPacket::serialize(prefix, offset, 0, 1, &payload);
        assert_eq!(serialized.len(), WEBRTC_PACKET_HEADER_LEN);

        let (d_prefix, d_offset, d_part_index, d_part_count, d_payload) = WebRtcPacket::deserialize(serialized);
        assert_eq!(d_prefix, prefix);
        assert_eq!(d_offset, offset);
        assert_eq!(d_part_index, 0);
        assert_eq!(d_part_count, 1);
        assert_eq!(d_payload, payload);
    }

    #[test]
    fn test_single_part_delimiter() {
        let prefix = 42u16;
        let offset = 0u64;
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let serialized = WebRtcPacket::serialize(prefix, offset, 0, 1, &payload);
        let (_, _, part_index, part_count, d_payload) = WebRtcPacket::deserialize(serialized);
        assert_eq!(part_index, 0);
        assert_eq!(part_count, 1);
        assert_eq!(d_payload, payload);
    }

    #[test]
    fn test_last_part_of_sixteen() {
        let prefix = 1u16;
        let offset = 128u64 * 1024;
        let payload = vec![0xAA; 3000];
        let serialized = WebRtcPacket::serialize(prefix, offset, 15, 16, &payload);
        let (_, d_offset, part_index, part_count, d_payload) = WebRtcPacket::deserialize(serialized);
        assert_eq!(d_offset, offset);
        assert_eq!(part_index, 15);
        assert_eq!(part_count, 16);
        assert_eq!(d_payload, payload);
    }
}
