//! TCP Protocol Handler for AetherXOS.

use crate::aether_packet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    LastAck,
    TimeWait,
}

aether_packet! {
    pub struct TcpHeader<'a> {
        src_port: u16;
        dst_port: u16;
        seq_num: u32;
        ack_num: u32;
        bitfield flags_block : u16 {
            data_offset: u16 = 12..16;
            reserved: u16 = 9..12;
            ns: u16 = 8..9;
            cwr: u16 = 7..8;
            ece: u16 = 6..7;
            urg: u16 = 5..6;
            ack: u16 = 4..5;
            psh: u16 = 3..4;
            rst: u16 = 2..3;
            syn: u16 = 1..2;
            fin: u16 = 0..1;
        }
        window_size: u16;
        checksum: u16;
        urgent_ptr: u16;
    }
}

impl<'a> TcpHeader<'a> {
    pub fn payload(&self) -> &'a [u8] {
        let offset = self.data_offset() as usize * 4;
        if offset <= self.as_bytes().len() {
            &self.as_bytes()[offset..]
        } else {
            &[]
        }
    }
}

pub struct TcpConnection {
    pub state: TcpState,
    pub local_port: u16,
    pub remote_port: u16,
    pub seq_nr: u32,
    pub ack_nr: u32,
    pub window_size: u32,      // Our receive window
    pub remote_window: u32,    // Peer's receive window
}

impl TcpConnection {
    /// Send data, respecting the sliding window flow control.
    pub fn send_data(&mut self, data: &[u8]) -> Result<usize, &'static str> {
        let available = self.remote_window as usize;
        let to_send = core::cmp::min(data.len(), available);
        
        if to_send == 0 {
            return Err("remote window full");
        }
        
        crate::klog_info!("[TCP] Sending {} bytes (Window: {})", to_send, available);
        self.seq_nr += to_send as u32;
        self.remote_window -= to_send as u32;
        
        Ok(to_send)
    }
    
    pub fn handle_segment(&mut self, header: &TcpHeader) {
        let seq = header.seq_num();
        let _ack = header.ack_num();
        
        match self.state {
            TcpState::Listen => {
                if header.syn() != 0 {
                    self.state = TcpState::SynReceived;
                    self.ack_nr = seq + 1;
                    crate::klog_info!("[TCP] Connection request (SYN) received. Moving to SYN_RECEIVED.");
                }
            }
            TcpState::SynReceived => {
                if header.ack() != 0 {
                    self.state = TcpState::Established;
                    crate::klog_info!("[TCP] Connection ESTABLISHED.");
                }
            }
            _ => {}
        }
    }
}
