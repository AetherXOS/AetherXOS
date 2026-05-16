use anyhow::Result;
use std::path::Path;
use crate::utils::logging;

pub struct RemoteCacheClient {
    pub base_url: String,
    pub client: reqwest::blocking::Client,
}

impl RemoteCacheClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn download(&self, key: &str, dest: &Path) -> Result<bool> {
        let url = format!("{}/{}", self.base_url, key);
        logging::debug("CACHE", &format!("Attempting to download artifact: {}", url), &[]);
        
        let mut response = self.client.get(url).send()?;
        if response.status().is_success() {
            let mut file = std::fs::File::create(dest)?;
            response.copy_to(&mut file)?;
            logging::success("CACHE", &format!("Artifact downloaded: {}", key), &[]);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn upload(&self, key: &str, src: &Path) -> Result<()> {
        let url = format!("{}/{}", self.base_url, key);
        logging::info("CACHE", &format!("Uploading artifact: {}", key), &[]);
        
        let file = std::fs::File::open(src)?;
        let response = self.client.put(url)
            .body(file)
            .send()?;
            
        if !response.status().is_success() {
            anyhow::bail!("Failed to upload artifact: {}", response.status());
        }
        
        logging::success("CACHE", "Upload complete", &[]);
        Ok(())
    }
}
