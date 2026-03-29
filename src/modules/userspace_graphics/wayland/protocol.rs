#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaylandMessageHeader {
    pub object_id: u32,
    pub opcode: u16,
    pub byte_len: u16,
}

pub fn parse_wire_header(bytes: &[u8]) -> Option<WaylandMessageHeader> {
    if bytes.len() < 8 {
        return None;
    }

    let object_id = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let word = u32::from_ne_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let opcode = (word & 0xFFFF) as u16;
    let byte_len = (word >> 16) as u16;

    if object_id == 0 || byte_len < 8 || (byte_len as usize) % 4 != 0 {
        return None;
    }

    Some(WaylandMessageHeader {
        object_id,
        opcode,
        byte_len,
    })
}

#[inline]
pub fn is_complete_frame(bytes: &[u8]) -> bool {
    let Some(header) = parse_wire_header(bytes) else {
        return false;
    };
    bytes.len() >= header.byte_len as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn parse_wayland_header_accepts_minimal_valid_frame() {
        let object_id = 1u32;
        let opcode = 0x21u16;
        let byte_len = 12u16;
        let word = ((byte_len as u32) << 16) | opcode as u32;
        let mut bytes = [0u8; 8];
        bytes[..4].copy_from_slice(&object_id.to_ne_bytes());
        bytes[4..].copy_from_slice(&word.to_ne_bytes());

        let hdr = parse_wire_header(&bytes).expect("valid wayland header");
        assert_eq!(hdr.object_id, object_id);
        assert_eq!(hdr.opcode, opcode);
        assert_eq!(hdr.byte_len, byte_len);
    }

    #[test_case]
    fn parse_wayland_header_rejects_invalid_size_alignment() {
        let object_id = 2u32;
        let opcode = 3u16;
        let byte_len = 10u16;
        let word = ((byte_len as u32) << 16) | opcode as u32;
        let mut bytes = [0u8; 8];
        bytes[..4].copy_from_slice(&object_id.to_ne_bytes());
        bytes[4..].copy_from_slice(&word.to_ne_bytes());

        assert!(parse_wire_header(&bytes).is_none());
    }

    #[test_case]
    fn complete_frame_requires_full_payload_length() {
        let object_id = 1u32;
        let opcode = 2u16;
        let byte_len = 16u16;
        let word = ((byte_len as u32) << 16) | opcode as u32;
        let mut bytes = [0u8; 16];
        bytes[..4].copy_from_slice(&object_id.to_ne_bytes());
        bytes[4..8].copy_from_slice(&word.to_ne_bytes());

        assert!(is_complete_frame(&bytes));
        assert!(!is_complete_frame(&bytes[..12]));
    }
}
