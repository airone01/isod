use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReleaseType {
    Stable,
    LTS, // Long Term Support
    Beta,
    Alpha,
    RC,       // Release Candidate
    Daily,    // Daily builds
    Weekly,   // Weekly builds
    Snapshot, // Development snapshots
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionInfo {
    pub version: String,
    pub release_type: ReleaseType,
    pub release_date: Option<String>,
    pub end_of_life: Option<String>,
    pub download_url_base: Option<String>,
    pub changelog_url: Option<String>,
    pub notes: Option<String>,
}

impl VersionInfo {
    pub fn new(version: &str, release_type: ReleaseType) -> Self {
        Self {
            version: version.to_string(),
            release_type,
            release_date: None,
            end_of_life: None,
            download_url_base: None,
            changelog_url: None,
            notes: None,
        }
    }

    pub fn with_release_date(mut self, date: &str) -> Self {
        self.release_date = Some(date.to_string());
        self
    }

    pub fn with_download_base(mut self, url: &str) -> Self {
        self.download_url_base = Some(url.to_string());
        self
    }

    pub fn with_changelog(mut self, url: &str) -> Self {
        self.changelog_url = Some(url.to_string());
        self
    }

    pub fn with_notes(mut self, notes: &str) -> Self {
        self.notes = Some(notes.to_string());
        self
    }

    /// Check if this version is still supported (not past EOL)
    pub fn is_supported(&self) -> bool {
        // If no EOL date is set, assume it's supported
        self.end_of_life.is_none()
        // TODO: Implement actual date comparison when we have a date parsing library
    }

    /// Parse version string into comparable components
    fn parse_version(&self) -> Vec<u32> {
        self.version
            .split(|c: char| c == '.' || c == '-' || c == '_')
            .filter_map(|part| {
                // Extract numeric part from strings like "24.04" or "rc1"
                let numeric_part: String =
                    part.chars().take_while(|c| c.is_ascii_digit()).collect();

                if numeric_part.is_empty() {
                    None
                } else {
                    numeric_part.parse().ok()
                }
            })
            .collect()
    }
}

impl PartialOrd for VersionInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VersionInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by release type priority
        let type_priority = |rt: &ReleaseType| -> u8 {
            match rt {
                ReleaseType::Stable => 100,
                ReleaseType::LTS => 110, // LTS is preferred over regular stable
                ReleaseType::RC => 80,
                ReleaseType::Beta => 60,
                ReleaseType::Alpha => 40,
                ReleaseType::Daily => 20,
                ReleaseType::Weekly => 25,
                ReleaseType::Snapshot => 10,
            }
        };

        // Compare release types first
        let type_cmp = type_priority(&self.release_type).cmp(&type_priority(&other.release_type));
        if type_cmp != Ordering::Equal {
            return type_cmp;
        }

        // If same release type, compare version numbers
        let self_parts = self.parse_version();
        let other_parts = other.parse_version();

        // Compare version parts
        for (self_part, other_part) in self_parts.iter().zip(other_parts.iter()) {
            match self_part.cmp(other_part) {
                Ordering::Equal => continue,
                other => return other,
            }
        }

        // If all compared parts are equal, the one with more parts is newer
        self_parts.len().cmp(&other_parts.len())
    }
}

impl fmt::Display for ReleaseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReleaseType::Stable => write!(f, "Stable"),
            ReleaseType::LTS => write!(f, "LTS"),
            ReleaseType::Beta => write!(f, "Beta"),
            ReleaseType::Alpha => write!(f, "Alpha"),
            ReleaseType::RC => write!(f, "RC"),
            ReleaseType::Daily => write!(f, "Daily"),
            ReleaseType::Weekly => write!(f, "Weekly"),
            ReleaseType::Snapshot => write!(f, "Snapshot"),
        }
    }
}

impl fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.version, self.release_type)?;
        if let Some(date) = &self.release_date {
            write!(f, " - {}", date)?;
        }
        Ok(())
    }
}

/// Trait for detecting available versions of a distribution
#[async_trait]
pub trait VersionDetector: Send + Sync + std::fmt::Debug {
    /// Detect all available versions
    async fn detect_versions(&self) -> Result<Vec<VersionInfo>>;

    /// Get the latest stable version
    async fn get_latest_stable(&self) -> Result<VersionInfo> {
        let versions = self.detect_versions().await?;
        versions
            .into_iter()
            .filter(|v| v.release_type == ReleaseType::Stable || v.release_type == ReleaseType::LTS)
            .max()
            .context("No stable versions found")
    }

    /// Check if a specific version exists
    async fn version_exists(&self, version: &str) -> Result<bool> {
        let versions = self.detect_versions().await?;
        Ok(versions.iter().any(|v| v.version == version))
    }
}

/// RSS/Atom feed based version detector
#[derive(Debug, Clone)]
pub struct FeedVersionDetector {
    pub feed_url: String,
    pub version_regex: String,
    pub release_type: ReleaseType,
}

#[async_trait]
impl VersionDetector for FeedVersionDetector {
    async fn detect_versions(&self) -> Result<Vec<VersionInfo>> {
        // TODO: Implement RSS/Atom feed parsing
        // This would use a feed parsing library to extract version information
        // from RSS feeds like Ubuntu's or Fedora's release announcements

        // Placeholder implementation
        Ok(vec![
            VersionInfo::new("placeholder", self.release_type.clone())
                .with_release_date("2024-01-01"),
        ])
    }
}

/// GitHub releases based version detector
#[derive(Debug, Clone)]
pub struct GitHubVersionDetector {
    pub repo_owner: String,
    pub repo_name: String,
    pub version_prefix: Option<String>,
    pub include_prereleases: bool,
}

#[async_trait]
impl VersionDetector for GitHubVersionDetector {
    async fn detect_versions(&self) -> Result<Vec<VersionInfo>> {
        // TODO: Implement GitHub API integration
        // This would use the GitHub API to fetch releases from a repository
        // https://api.github.com/repos/{owner}/{repo}/releases

        // Placeholder implementation
        Ok(vec![
            VersionInfo::new("v1.0.0", ReleaseType::Stable).with_release_date("2024-01-01"),
        ])
    }
}

/// Web scraping based version detector
#[derive(Debug, Clone)]
pub struct WebScrapingDetector {
    pub base_url: String,
    pub version_selector: String, // CSS selector or XPath
    pub version_regex: String,    // Regex to extract version from text
    pub date_selector: Option<String>,
    pub date_format: Option<String>,
}

#[async_trait]
impl VersionDetector for WebScrapingDetector {
    async fn detect_versions(&self) -> Result<Vec<VersionInfo>> {
        // TODO: Implement web scraping with HTML parsing
        // This would fetch HTML pages and extract version information
        // using CSS selectors or XPath expressions

        // Placeholder implementation
        Ok(vec![VersionInfo::new("scraped-1.0", ReleaseType::Stable)])
    }
}

/// API-based version detector for distributions with APIs
#[derive(Debug, Clone)]
pub struct ApiVersionDetector {
    pub api_url: String,
    pub auth_header: Option<String>,
    pub version_json_path: String, // JSONPath to version field
    pub date_json_path: Option<String>,
}

#[async_trait]
impl VersionDetector for ApiVersionDetector {
    async fn detect_versions(&self) -> Result<Vec<VersionInfo>> {
        // TODO: Implement API-based version detection
        // This would make HTTP requests to distribution APIs
        // and parse JSON responses to extract version information

        // Placeholder implementation
        Ok(vec![VersionInfo::new("api-1.0", ReleaseType::Stable)])
    }
}

/// Static version detector for distributions with known, infrequent releases
#[derive(Debug, Clone)]
pub struct StaticVersionDetector {
    pub versions: Vec<VersionInfo>,
}

#[async_trait]
impl VersionDetector for StaticVersionDetector {
    async fn detect_versions(&self) -> Result<Vec<VersionInfo>> {
        Ok(self.versions.clone())
    }
}

/// Composite version detector that tries multiple detection methods
pub struct CompositeVersionDetector {
    pub detectors: Vec<Box<dyn VersionDetector>>,
}

impl std::fmt::Debug for CompositeVersionDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeVersionDetector")
            .field("detectors", &format!("{} detectors", self.detectors.len()))
            .finish()
    }
}

#[async_trait]
impl VersionDetector for CompositeVersionDetector {
    async fn detect_versions(&self) -> Result<Vec<VersionInfo>> {
        let mut all_versions = Vec::new();

        for detector in &self.detectors {
            match detector.detect_versions().await {
                Ok(mut versions) => {
                    all_versions.append(&mut versions);
                }
                Err(e) => {
                    // Log error but continue with other detectors
                    eprintln!("Version detector failed: {}", e);
                }
            }
        }

        // Remove duplicates and sort
        all_versions.sort_by(|a, b| b.cmp(a)); // Newest first
        all_versions.dedup_by(|a, b| a.version == b.version);

        Ok(all_versions)
    }
}

impl CompositeVersionDetector {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    pub fn add_detector(mut self, detector: Box<dyn VersionDetector>) -> Self {
        self.detectors.push(detector);
        self
    }
}

/// Helper functions for creating common version detectors
pub mod detectors {
    use super::*;

    /// Create a GitHub releases detector
    pub fn github(owner: &str, repo: &str, include_prereleases: bool) -> Box<dyn VersionDetector> {
        Box::new(GitHubVersionDetector {
            repo_owner: owner.to_string(),
            repo_name: repo.to_string(),
            version_prefix: None,
            include_prereleases,
        })
    }

    /// Create an RSS feed detector
    pub fn rss_feed(
        feed_url: &str,
        version_regex: &str,
        release_type: ReleaseType,
    ) -> Box<dyn VersionDetector> {
        Box::new(FeedVersionDetector {
            feed_url: feed_url.to_string(),
            version_regex: version_regex.to_string(),
            release_type,
        })
    }

    /// Create a web scraping detector
    pub fn web_scraper(
        base_url: &str,
        version_selector: &str,
        version_regex: &str,
    ) -> Box<dyn VersionDetector> {
        Box::new(WebScrapingDetector {
            base_url: base_url.to_string(),
            version_selector: version_selector.to_string(),
            version_regex: version_regex.to_string(),
            date_selector: None,
            date_format: None,
        })
    }

    /// Create an API detector
    pub fn api(api_url: &str, version_json_path: &str) -> Box<dyn VersionDetector> {
        Box::new(ApiVersionDetector {
            api_url: api_url.to_string(),
            auth_header: None,
            version_json_path: version_json_path.to_string(),
            date_json_path: None,
        })
    }

    /// Create a static detector with predefined versions
    pub fn static_versions(versions: Vec<VersionInfo>) -> Box<dyn VersionDetector> {
        Box::new(StaticVersionDetector { versions })
    }

    /// Create a composite detector
    pub fn composite() -> CompositeVersionDetector {
        CompositeVersionDetector::new()
    }
}
