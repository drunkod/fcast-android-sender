//! Binary wire format for the WebSocket `push_buffer` frame.
//!
//! Layout (all integers big-endian):
//!
//! ```text
//!   u32 pipeline_id_len | pipeline_id_bytes
//!   u32 element_name_len | element_name_bytes
//!   u64 pts_ns
//!   payload (rest of frame)
//! ```
//!
//! The header is `4 + pid_len + 4 + elem_len + 8` bytes; the payload follows.
//! Strings are UTF-8 and not null-terminated. Length prefixes are unsigned and
//! interpreted as byte counts, not character counts.

pub const MIN_HEADER_BYTES: usize = 4 + 4 + 8;

#[derive(Debug)]
pub struct DecodedPushBuffer<'a> {
    pub pipeline_id: &'a str,
    pub element_name: &'a str,
    pub pts_ns: u64,
    pub payload: &'a [u8],
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    TooShort,
    BadPidLen,
    BadElemLen,
    PidNotUtf8,
    ElemNotUtf8,
}

pub fn encode(pipeline_id: &str, element_name: &str, pts_ns: u64, payload: &[u8]) -> Vec<u8> {
    let pid = pipeline_id.as_bytes();
    let elem = element_name.as_bytes();
    let mut buf = Vec::with_capacity(MIN_HEADER_BYTES + pid.len() + elem.len() + payload.len());
    buf.extend_from_slice(&(pid.len() as u32).to_be_bytes());
    buf.extend_from_slice(pid);
    buf.extend_from_slice(&(elem.len() as u32).to_be_bytes());
    buf.extend_from_slice(elem);
    buf.extend_from_slice(&pts_ns.to_be_bytes());
    buf.extend_from_slice(payload);
    buf
}

pub fn decode(bin: &[u8]) -> Result<DecodedPushBuffer<'_>, DecodeError> {
    if bin.len() < MIN_HEADER_BYTES {
        return Err(DecodeError::TooShort);
    }

    let pid_len = u32::from_be_bytes(bin[0..4].try_into().unwrap()) as usize;
    let mut offset = 4;
    if offset + pid_len + 4 > bin.len() {
        return Err(DecodeError::BadPidLen);
    }
    let pipeline_id =
        std::str::from_utf8(&bin[offset..offset + pid_len]).map_err(|_| DecodeError::PidNotUtf8)?;
    offset += pid_len;

    let elem_len = u32::from_be_bytes(bin[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    if offset + elem_len + 8 > bin.len() {
        return Err(DecodeError::BadElemLen);
    }
    let element_name = std::str::from_utf8(&bin[offset..offset + elem_len])
        .map_err(|_| DecodeError::ElemNotUtf8)?;
    offset += elem_len;

    let pts_ns = u64::from_be_bytes(bin[offset..offset + 8].try_into().unwrap());
    offset += 8;

    Ok(DecodedPushBuffer {
        pipeline_id,
        element_name,
        pts_ns,
        payload: &bin[offset..],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let payload = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let wire = encode("pipe-42", "camera-src", 0x0123_4567_89AB_CDEF, &payload);
        let decoded = decode(&wire).expect("decode");
        assert_eq!(decoded.pipeline_id, "pipe-42");
        assert_eq!(decoded.element_name, "camera-src");
        assert_eq!(decoded.pts_ns, 0x0123_4567_89AB_CDEF);
        assert_eq!(decoded.payload, payload.as_slice());
    }

    #[test]
    fn round_trip_empty_payload() {
        let wire = encode("p", "e", 0, &[]);
        let decoded = decode(&wire).expect("decode");
        assert_eq!(decoded.pipeline_id, "p");
        assert_eq!(decoded.element_name, "e");
        assert_eq!(decoded.pts_ns, 0);
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn too_short_header() {
        assert!(matches!(decode(&[0u8; 4]), Err(DecodeError::TooShort)));
    }

    #[test]
    fn pid_len_overruns_buffer() {
        let mut wire = Vec::new();
        wire.extend_from_slice(&100u32.to_be_bytes()); // pid_len = 100
        wire.extend_from_slice(b"only-three");
        wire.extend_from_slice(&0u32.to_be_bytes());
        wire.extend_from_slice(&0u64.to_be_bytes());
        assert!(matches!(decode(&wire), Err(DecodeError::BadPidLen)));
    }

    #[test]
    fn pid_not_utf8() {
        let mut wire = Vec::new();
        wire.extend_from_slice(&2u32.to_be_bytes());
        wire.extend_from_slice(&[0xff, 0xfe]);
        wire.extend_from_slice(&0u32.to_be_bytes());
        wire.extend_from_slice(&0u64.to_be_bytes());
        assert!(matches!(decode(&wire), Err(DecodeError::PidNotUtf8)));
    }
}
