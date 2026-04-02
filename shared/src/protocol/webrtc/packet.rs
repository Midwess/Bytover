pub struct WebRtcPacket;

impl WebRtcPacket {
    /// Serializes a prefix, offset and payload into a byte vector.
    /// Uses unsafe for maximum performance by avoiding multiple reallocations and checks.
    #[inline]
    pub fn serialize(prefix: u16, offset: u64, payload: &[u8]) -> Vec<u8> {
        let len = payload.len();
        let mut vec = Vec::with_capacity(len + 10);
        unsafe {
            vec.set_len(len + 10);
            let ptr = vec.as_mut_ptr();
            // Copy 2-byte prefix (Little Endian)
            std::ptr::copy_nonoverlapping(prefix.to_le_bytes().as_ptr(), ptr, 2);
            // Copy 8-byte offset (Little Endian)
            std::ptr::copy_nonoverlapping(offset.to_le_bytes().as_ptr(), ptr.add(2), 8);
            // Copy payload
            std::ptr::copy_nonoverlapping(payload.as_ptr(), ptr.add(10), len);
        }
        vec
    }

    /// Deserializes a byte vector into a (prefix, offset, payload) tuple.
    /// Uses unsafe to shift the payload in-place and modify the vector's length,
    /// avoiding a second allocation for the payload.
    #[inline]
    pub fn deserialize(mut data: Vec<u8>) -> (u16, u64, Vec<u8>) {
        if data.len() < 10 {
            return (0, 0, data);
        }
        unsafe {
            // Read prefix from the first 2 bytes
            let prefix = u16::from_le_bytes(*(data.as_ptr() as *const [u8; 2]));
            // Read offset from the next 8 bytes (Starting at index 2)
            let offset = u64::from_le_bytes(*(data.as_ptr().add(2) as *const [u8; 8]));
            
            let len = data.len();
            // Shift the rest of the data 10 bytes to the left
            std::ptr::copy(data.as_ptr().add(10), data.as_mut_ptr(), len - 10);
            // Adjust length to exclude the prefix and offset
            data.set_len(len - 10);
            
            (prefix, offset, data)
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
        let payload = vec![1, 2, 3, 4, 5];
        let serialized = WebRtcPacket::serialize(prefix, offset, &payload);
        assert_eq!(serialized.len(), 15);
        // Little endian prefix check
        assert_eq!(serialized[0], 0x34);
        assert_eq!(serialized[1], 0x12);
        // Little endian offset check
        assert_eq!(serialized[2], 0x88);
        assert_eq!(serialized[9], 0x11);
        assert_eq!(&serialized[10..], &payload[..]);
        
        let (d_prefix, d_offset, d_payload) = WebRtcPacket::deserialize(serialized);
        assert_eq!(d_prefix, prefix);
        assert_eq!(d_offset, offset);
        assert_eq!(d_payload, payload);
    }

    #[test]
    fn test_empty_payload() {
        let prefix = 0xFFFFu16;
        let offset = 0u64;
        let payload = vec![];
        let serialized = WebRtcPacket::serialize(prefix, offset, &payload);
        assert_eq!(serialized.len(), 10);
        
        let (d_prefix, d_offset, d_payload) = WebRtcPacket::deserialize(serialized);
        assert_eq!(d_prefix, prefix);
        assert_eq!(d_offset, offset);
        assert_eq!(d_payload, payload);
    }
}
