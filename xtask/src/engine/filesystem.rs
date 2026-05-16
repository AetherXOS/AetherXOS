use std::path::PathBuf;
use crate::utils::core::context;

pub struct PathMapper {
    pub base_dir: PathBuf,
}

impl PathMapper {
    pub fn new() -> Self {
        Self { base_dir: context::out_dir() }
    }

    pub fn stage(&self, name: &str) -> PathBuf {
        self.base_dir.join("stage").join(name)
    }

    pub fn artifact(&self, name: &str) -> PathBuf {
        self.base_dir.join(name)
    }
}
