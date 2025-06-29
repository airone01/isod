pub mod config;
pub mod download;
pub mod registry;
pub mod usb;

// Re-export commonly used types for easier access in tests
pub use config::ConfigManager;
pub use download::{
    ChecksumType, ChecksumVerifier, DownloadManager, DownloadOptions, DownloadProgress,
    DownloadRequest,
};
pub use registry::IsoRegistry;

// Re-export registry types
pub use registry::sources::{SourcePriority, SourceType};
pub use registry::{DownloadSource, IsoInfo, ReleaseType, VersionInfo};
