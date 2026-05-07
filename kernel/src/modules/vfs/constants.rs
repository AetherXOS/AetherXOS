//! VFS magic constants and file type definitions
//!
//! Centralizes all magic values, mode bits, and type codes used across the filesystem.

// ─────────────────────────────────────────────────────────────────────────────
// File Mode Bits
// ─────────────────────────────────────────────────────────────────────────────

pub const MODE_DIR: u32 = 0o040000;           // Directory
pub const MODE_REG: u32 = 0o100000;           // Regular file
pub const MODE_LNK: u32 = 0o120000;           // Symlink
pub const MODE_TYPE_MASK: u32 = 0o170000;    // Mask to extract file type
pub const MODE_PERMS_MASK: u16 = 0o7777;     // Mask to extract permissions

pub const MODE_RWXRWXRWX: u32 = 0o777;       // All permissions
pub const MODE_STICKY_BIT: u32 = 0o01000;    // Sticky bit (for /tmp dirs)
pub const MODE_DIR_DEFAULT: u32 = 0o040755;  // Default directory mode (rwxr-xr-x + dir)
pub const MODE_DIR_STICKY: u32 = 0o040777 | MODE_STICKY_BIT; // /tmp-style directory
pub const MODE_FILE_DEFAULT: u32 = 0o100644; // Default file mode (rw-r--r--)

// ─────────────────────────────────────────────────────────────────────────────
// Directory Entry Types (dirent d_type)
// ─────────────────────────────────────────────────────────────────────────────

pub const DT_DIR: u8 = 4;   // Directory
pub const DT_REG: u8 = 8;   // Regular file
pub const DT_LNK: u8 = 10;  // Symbolic link

// ─────────────────────────────────────────────────────────────────────────────
// Symlink Resolution Limits
// ─────────────────────────────────────────────────────────────────────────────

pub const SYMLINK_MAX_DEPTH: usize = 16; // Max symlinks to follow before ELOOP

// ─────────────────────────────────────────────────────────────────────────────
// Page/Block Sizes
// ─────────────────────────────────────────────────────────────────────────────

pub const PAGE_SIZE: usize = 4096;          // Standard page size
pub const BLOCK_SIZE: usize = 4096;         // Standard block size for file stats
pub const BLOCK_SHIFT: usize = 512;         // Used for block count calculations

// ─────────────────────────────────────────────────────────────────────────────
// File Descriptor Limits
// ─────────────────────────────────────────────────────────────────────────────

pub const FD_TABLE_SIZE: u32 = 1024; // Max open file descriptors per process

// ─────────────────────────────────────────────────────────────────────────────
// Path Helpers
// ─────────────────────────────────────────────────────────────────────────────

pub const MAX_PATH_LEN: usize = 4096;      // Max path length
pub const MAX_FILENAME_LEN: usize = 255;   // Max single component length
