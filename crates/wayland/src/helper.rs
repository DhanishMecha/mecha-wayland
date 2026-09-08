use crate::{ObjectId, RawWaylandEvent};

const HEADER: usize = 8;

/// Splits `bytes` into whole messages and the tail that is not one yet. The
/// tail is kept for the next read: a message may straddle two reads.
pub fn parse_messages(bytes: Vec<u8>) -> (Vec<RawWaylandEvent>, Vec<u8>) {
    let mut events = Vec::new();
    let mut offset = 0;
    while offset + HEADER <= bytes.len() {
        let sender_id = u32::from_ne_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let word2 = u32::from_ne_bytes(bytes[offset + 4..offset + 8].try_into().unwrap());
        let size = (word2 >> 16) as usize;
        let opcode = word2 & 0xffff;
        assert!(size >= HEADER, "malformed wayland message: size {size}");
        if offset + size > bytes.len() {
            break;
        }
        events.push(RawWaylandEvent {
            object_id: ObjectId(sender_id),
            opcode,
            data: bytes[offset + HEADER..offset + size].to_vec(),
        });
        offset += size;
    }
    (events, bytes[offset..].to_vec())
}

pub fn encode_string(buf: &mut Vec<u8>, s: &str) {
    let str_len = s.len() + 1; // includes null terminator
    let padded = (str_len + 3) & !3;
    buf.extend_from_slice(&(str_len as u32).to_ne_bytes());
    buf.extend_from_slice(s.as_bytes());
    buf.push(0);
    buf.extend(std::iter::repeat_n(0u8, padded - str_len));
}
