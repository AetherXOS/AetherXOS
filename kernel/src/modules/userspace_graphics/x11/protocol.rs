#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X11SetupPrefix {
    pub byte_order: u8,
    pub major_version: u16,
    pub minor_version: u16,
    pub auth_proto_name_len: u16,
    pub auth_proto_data_len: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X11RequestPrefix {
    pub opcode: u8,
    pub payload_words: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X11ReplyPrefix {
    pub status: u8,
    pub sequence: u16,
    pub payload_words: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X11ServerPacketPrefix {
    Error { code: u8, sequence: u16 },
    Reply(X11ReplyPrefix),
    Event { event_code: u8, sequence: u16 },
}

pub fn parse_setup_prefix(bytes: &[u8]) -> Option<X11SetupPrefix> {
    if bytes.len() < 12 {
        return None;
    }

    let byte_order = bytes[0];
    if byte_order != b'l' && byte_order != b'B' {
        return None;
    }

    let read_u16 = |off: usize| -> u16 {
        if byte_order == b'l' {
            u16::from_le_bytes([bytes[off], bytes[off + 1]])
        } else {
            u16::from_be_bytes([bytes[off], bytes[off + 1]])
        }
    };

    let major_version = read_u16(2);
    let minor_version = read_u16(4);
    let auth_proto_name_len = read_u16(6);
    let auth_proto_data_len = read_u16(8);

    if major_version != 11 {
        return None;
    }

    Some(X11SetupPrefix {
        byte_order,
        major_version,
        minor_version,
        auth_proto_name_len,
        auth_proto_data_len,
    })
}

#[inline]
fn pad4_len(v: usize) -> usize {
    (v + 3) & !3
}

#[inline]
pub fn setup_total_len(prefix: &X11SetupPrefix) -> usize {
    12 + pad4_len(prefix.auth_proto_name_len as usize) + pad4_len(prefix.auth_proto_data_len as usize)
}

#[inline]
pub fn has_complete_setup_request(bytes: &[u8]) -> bool {
    let Some(prefix) = parse_setup_prefix(bytes) else {
        return false;
    };
    bytes.len() >= setup_total_len(&prefix)
}

#[inline]
pub fn parse_request_prefix(bytes: &[u8], byte_order: u8) -> Option<X11RequestPrefix> {
    if bytes.len() < 4 {
        return None;
    }
    if byte_order != b'l' && byte_order != b'B' {
        return None;
    }

    let opcode = bytes[0];
    let payload_words = if byte_order == b'l' {
        u16::from_le_bytes([bytes[2], bytes[3]])
    } else {
        u16::from_be_bytes([bytes[2], bytes[3]])
    };

    if opcode == 0 || payload_words == 0 {
        return None;
    }

    Some(X11RequestPrefix {
        opcode,
        payload_words,
    })
}

#[inline]
pub fn has_complete_request(bytes: &[u8], byte_order: u8) -> bool {
    let Some(prefix) = parse_request_prefix(bytes, byte_order) else {
        return false;
    };
    bytes.len() >= (prefix.payload_words as usize) * 4
}

#[inline]
pub fn parse_reply_prefix(bytes: &[u8], byte_order: u8) -> Option<X11ReplyPrefix> {
    if bytes.len() < 32 {
        return None;
    }
    if byte_order != b'l' && byte_order != b'B' {
        return None;
    }

    let status = bytes[0];
    // Setup success replies start with 1; this keeps checks conservative.
    if status != 1 {
        return None;
    }

    let sequence = if byte_order == b'l' {
        u16::from_le_bytes([bytes[2], bytes[3]])
    } else {
        u16::from_be_bytes([bytes[2], bytes[3]])
    };
    let payload_words = if byte_order == b'l' {
        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]])
    } else {
        u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]])
    };

    Some(X11ReplyPrefix {
        status,
        sequence,
        payload_words,
    })
}

#[inline]
pub fn has_complete_reply(bytes: &[u8], byte_order: u8) -> bool {
    let Some(prefix) = parse_reply_prefix(bytes, byte_order) else {
        return false;
    };
    let total = 32usize.saturating_add((prefix.payload_words as usize).saturating_mul(4));
    bytes.len() >= total
}

#[inline]
pub fn parse_server_packet_prefix(bytes: &[u8], byte_order: u8) -> Option<X11ServerPacketPrefix> {
    if bytes.len() < 32 {
        return None;
    }
    if byte_order != b'l' && byte_order != b'B' {
        return None;
    }

    let sequence = if byte_order == b'l' {
        u16::from_le_bytes([bytes[2], bytes[3]])
    } else {
        u16::from_be_bytes([bytes[2], bytes[3]])
    };

    match bytes[0] {
        0 => Some(X11ServerPacketPrefix::Error {
            code: bytes[1],
            sequence,
        }),
        1 => parse_reply_prefix(bytes, byte_order).map(X11ServerPacketPrefix::Reply),
        event_code => Some(X11ServerPacketPrefix::Event {
            event_code,
            sequence,
        }),
    }
}

#[inline]
pub fn has_complete_server_packet(bytes: &[u8], byte_order: u8) -> bool {
    let Some(prefix) = parse_server_packet_prefix(bytes, byte_order) else {
        return false;
    };
    match prefix {
        X11ServerPacketPrefix::Reply(reply) => {
            let total = 32usize.saturating_add((reply.payload_words as usize).saturating_mul(4));
            bytes.len() >= total
        }
        X11ServerPacketPrefix::Error { .. } | X11ServerPacketPrefix::Event { .. } => {
            bytes.len() >= 32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn parse_x11_setup_prefix_accepts_little_endian_v11() {
        let bytes = [
            b'l', 0, // order + pad
            11, 0, // major
            0, 0, // minor
            18, 0, // auth name len
            16, 0, // auth data len
            0, 0, // pad
        ];
        let setup = parse_setup_prefix(&bytes).expect("x11 setup prefix");
        assert_eq!(setup.major_version, 11);
        assert_eq!(setup.auth_proto_name_len, 18);
    }

    #[test_case]
    fn parse_x11_setup_prefix_rejects_non_x11_major() {
        let bytes = [
            b'l', 0, // order + pad
            10, 0, // major
            0, 0, // minor
            0, 0, // auth name len
            0, 0, // auth data len
            0, 0, // pad
        ];
        assert!(parse_setup_prefix(&bytes).is_none());
    }

    #[test_case]
    fn complete_x11_setup_requires_padded_payload() {
        let mut bytes = [0u8; 48];
        bytes[0] = b'l';
        bytes[2..4].copy_from_slice(&11u16.to_le_bytes());
        bytes[4..6].copy_from_slice(&0u16.to_le_bytes());
        bytes[6..8].copy_from_slice(&18u16.to_le_bytes());
        bytes[8..10].copy_from_slice(&16u16.to_le_bytes());

        assert!(has_complete_setup_request(&bytes));
        assert!(!has_complete_setup_request(&bytes[..20]));
    }

    #[test_case]
    fn parse_x11_request_prefix_accepts_basic_core_request() {
        // CreateWindow-like header: opcode=1, request length=2 words.
        let bytes = [1u8, 0, 2, 0, 0, 0, 0, 0];
        let req = parse_request_prefix(&bytes, b'l').expect("x11 request prefix");
        assert_eq!(req.opcode, 1);
        assert_eq!(req.payload_words, 2);
        assert!(has_complete_request(&bytes, b'l'));
    }

    #[test_case]
    fn parse_x11_request_prefix_rejects_zero_opcode_or_len() {
        let zero_opcode = [0u8, 0, 1, 0];
        let zero_len = [1u8, 0, 0, 0];
        assert!(parse_request_prefix(&zero_opcode, b'l').is_none());
        assert!(parse_request_prefix(&zero_len, b'l').is_none());
    }

    #[test_case]
    fn parse_x11_reply_prefix_accepts_success_header() {
        let mut bytes = [0u8; 40];
        bytes[0] = 1; // success
        bytes[2..4].copy_from_slice(&9u16.to_le_bytes());
        bytes[4..8].copy_from_slice(&2u32.to_le_bytes()); // 8-byte payload after 32-byte header
        let reply = parse_reply_prefix(&bytes, b'l').expect("x11 reply prefix");
        assert_eq!(reply.sequence, 9);
        assert_eq!(reply.payload_words, 2);
        assert!(has_complete_reply(&bytes, b'l'));
    }

    #[test_case]
    fn parse_x11_reply_prefix_rejects_non_success_status() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0;
        assert!(parse_reply_prefix(&bytes, b'l').is_none());
    }

    #[test_case]
    fn parse_server_packet_prefix_accepts_event_and_error_packets() {
        let mut event = [0u8; 32];
        event[0] = 2;
        event[2..4].copy_from_slice(&7u16.to_le_bytes());
        assert_eq!(
            parse_server_packet_prefix(&event, b'l'),
            Some(X11ServerPacketPrefix::Event {
                event_code: 2,
                sequence: 7,
            })
        );

        let mut error = [0u8; 32];
        error[0] = 0;
        error[1] = 3;
        error[2..4].copy_from_slice(&8u16.to_le_bytes());
        assert_eq!(
            parse_server_packet_prefix(&error, b'l'),
            Some(X11ServerPacketPrefix::Error {
                code: 3,
                sequence: 8,
            })
        );
    }

    #[test_case]
    fn has_complete_server_packet_requires_full_reply_payload() {
        let mut reply = [0u8; 36];
        reply[0] = 1;
        reply[2..4].copy_from_slice(&1u16.to_le_bytes());
        reply[4..8].copy_from_slice(&2u32.to_le_bytes());
        assert!(!has_complete_server_packet(&reply, b'l'));

        let full = [0u8; 40];
        let mut full_reply = full;
        full_reply[0] = 1;
        full_reply[2..4].copy_from_slice(&1u16.to_le_bytes());
        full_reply[4..8].copy_from_slice(&2u32.to_le_bytes());
        assert!(has_complete_server_packet(&full_reply, b'l'));
    }
}
