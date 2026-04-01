pub(super) const VIRTIO_REG_DEVICE_FEATURES: u16 = 0x00;
pub(super) const VIRTIO_REG_GUEST_FEATURES: u16 = 0x04;
pub(super) const VIRTIO_REG_QUEUE_ADDRESS: u16 = 0x08;
pub(super) const VIRTIO_REG_QUEUE_SIZE: u16 = 0x0C;
pub(super) const VIRTIO_REG_QUEUE_SELECT: u16 = 0x0E;
pub(super) const VIRTIO_REG_QUEUE_NOTIFY: u16 = 0x10;
pub(super) const VIRTIO_REG_DEVICE_STATUS: u16 = 0x12;
pub(super) const VIRTIO_REG_ISR_STATUS: u16 = 0x13;

pub(super) const STATUS_ACKNOWLEDGE: u8 = 1;
pub(super) const STATUS_DRIVER: u8 = 2;
pub(super) const STATUS_DRIVER_OK: u8 = 4;
pub(super) const STATUS_FAILED: u8 = 128;
pub(super) const STATUS_FEATURES_OK: u8 = 8;

pub(super) const MAX_SOFTWARE_BUDGET: usize = 1024;

pub(super) const VIRTQ_DESC_F_NEXT: u16 = 1;
pub(super) const VIRTQ_DESC_F_WRITE: u16 = 2;

pub(super) const VIRTIO_QUEUE_RX: u16 = 0;
pub(super) const VIRTIO_QUEUE_TX: u16 = 1;
pub(super) const VIRTIO_QUEUE_CTRL: u16 = 2;
pub(super) const VIRTIO_QUEUE_MAX_SIZE: u16 = 256;
pub(super) const VIRTIO_QUEUE_ALIGN: usize = 4096;
pub(super) const VIRTIO_QUEUE_MEMORY_BYTES: usize = 32 * 1024;

pub(super) const VIRTIO_NET_HDR_BYTES: usize = 10;
pub(super) const VIRTIO_RX_BUFFER_BYTES: usize = 2048;
pub(super) const VIRTIO_CTRL_MAX_CMD_BYTES: usize = 64;
pub(super) const VIRTIO_CTRL_TIMEOUT_SPINS: usize = 100_000;
pub(super) const VIRTIO_CTRL_STATUS_OK: u8 = 0;

pub(super) const VIRTIO_NET_CONFIG_MAC_OFFSET: u16 = 0x14;
pub(super) const VIRTIO_NET_F_MAC: u32 = 1 << 5;
pub(super) const VIRTIO_NET_F_CTRL_VQ: u32 = 1 << 17;
pub(super) const VIRTIO_NET_F_CTRL_RX: u32 = 1 << 18;

pub(super) const VIRTIO_NET_CTRL_RX_CLASS: u8 = 0;
pub(super) const VIRTIO_NET_CTRL_RX_PROMISC_CMD: u8 = 0;
pub(super) const VIRTIO_NET_CTRL_MAC_CLASS: u8 = 1;
pub(super) const VIRTIO_NET_CTRL_MAC_ADDR_SET_CMD: u8 = 1;
