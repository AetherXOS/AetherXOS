use alloc::vec::Vec;
use alloc::string::String;

pub struct FileFixture {
    pub path: String,
    pub content: Vec<u8>,
    pub permissions: u32,
}

impl FileFixture {
    pub fn new(path: &str, content: &[u8]) -> Self {
        Self {
            path: String::from(path),
            content: Vec::from(content),
            permissions: 0o644,
        }
    }

    pub fn with_permissions(mut self, perms: u32) -> Self {
        self.permissions = perms;
        self
    }

    pub fn size(&self) -> usize {
        self.content.len()
    }

    pub fn read(&self) -> &[u8] {
        &self.content
    }

    pub fn write(&mut self, data: &[u8]) {
        self.content.clear();
        self.content.extend_from_slice(data);
    }

    pub fn append(&mut self, data: &[u8]) {
        self.content.extend_from_slice(data);
    }

    pub fn clear(&mut self) {
        self.content.clear();
    }
}

pub fn create_empty_file(path: &str) -> FileFixture {
    FileFixture::new(path, &[])
}

pub fn create_text_file(path: &str, content: &str) -> FileFixture {
    FileFixture::new(path, content.as_bytes())
}

pub fn create_binary_file(path: &str, size: usize) -> FileFixture {
    let mut content = Vec::with_capacity(size);
    for i in 0..size {
        content.push((i % 256) as u8);
    }
    FileFixture::new(path, &content)
}

pub fn create_random_file(path: &str, size: usize, seed: u64) -> FileFixture {
    let mut content = Vec::with_capacity(size);
    let mut state = seed;
    for _ in 0..size {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        content.push((state >> 33) as u8);
    }
    FileFixture::new(path, &content)
}

pub const TEST_FILE_PATHS: &[&str] = &[
    "/test/file1.txt",
    "/test/file2.bin",
    "/data/config.json",
    "/tmp/temp.dat",
    "/var/log/test.log",
];

pub const TEST_FILE_CONTENTS: &[(&str, &[u8])] = &[
    ("hello.txt", b"Hello, World!"),
    ("empty.bin", &[]),
    ("binary.dat", &[0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD, 0xFC]),
    ("config.json", b"{\"key\": \"value\"}"),
];
