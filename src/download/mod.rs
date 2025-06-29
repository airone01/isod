pub mod checksum;
pub mod engine;
pub mod manager;
pub mod progress;
pub mod torrent;

pub use checksum::{ChecksumType, ChecksumVerifier};
pub use engine::{DownloadEngine, DownloadTask};
pub use manager::{DownloadManager, DownloadOptions};
pub use progress::DownloadProgress;

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub url: String,
    pub output_path: PathBuf,
    pub expected_checksum: Option<String>,
    pub checksum_type: Option<ChecksumType>,
    pub user_agent: Option<String>,
    pub resume: bool,
}

impl DownloadRequest {
    pub fn new(url: String, output_path: PathBuf) -> Self {
        Self {
            url,
            output_path,
            expected_checksum: None,
            checksum_type: None,
            user_agent: Some("isod/0.1.0".to_string()),
            resume: true,
        }
    }

    pub fn with_checksum(mut self, checksum: String, checksum_type: ChecksumType) -> Self {
        self.expected_checksum = Some(checksum);
        self.checksum_type = Some(checksum_type);
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    pub fn no_resume(mut self) -> Self {
        self.resume = false;
        self
    }
}
