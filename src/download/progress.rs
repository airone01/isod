use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum DownloadProgress {
    Started {
        id: String,
        url: String,
        output_path: PathBuf,
    },
    Progress {
        id: String,
        bytes_downloaded: u64,
        total_bytes: u64,
        progress_percent: u8,
        speed_bps: u64,
    },
    VerifyingChecksum {
        id: String,
    },
    ChecksumVerified {
        id: String,
    },
    ChecksumFailed {
        id: String,
        expected: String,
    },
    Completed {
        id: String,
        bytes_downloaded: u64,
        checksum_verified: bool,
    },
    Failed {
        id: String,
        error: String,
        attempts: u32,
    },
    Retry {
        id: String,
        attempt: u32,
        max_attempts: u32,
        delay: Duration,
    },
    Cancelled {
        id: String,
    },
    Error {
        id: String,
        error: String,
    },
}

pub struct ProgressTracker {
    // Could add methods to track multiple downloads, calculate ETA, etc.
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {}
    }

    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    pub fn format_speed(bytes_per_second: u64) -> String {
        format!("{}/s", Self::format_bytes(bytes_per_second))
    }

    pub fn calculate_eta(
        bytes_downloaded: u64,
        total_bytes: u64,
        speed_bps: u64,
    ) -> Option<Duration> {
        if speed_bps == 0 || total_bytes == 0 || bytes_downloaded >= total_bytes {
            return None;
        }

        let remaining_bytes = total_bytes - bytes_downloaded;
        let eta_seconds = remaining_bytes / speed_bps;
        Some(Duration::from_secs(eta_seconds))
    }

    pub fn format_duration(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}
