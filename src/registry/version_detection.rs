use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::time::Duration;

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
    client: Client,
}

impl FeedVersionDetector {
    pub fn new(feed_url: String, version_regex: String, release_type: ReleaseType) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("isod/0.1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            feed_url,
            version_regex,
            release_type,
            client,
        }
    }
}

#[async_trait]
impl VersionDetector for FeedVersionDetector {
    async fn detect_versions(&self) -> Result<Vec<VersionInfo>> {
        let response = self
            .client
            .get(&self.feed_url)
            .send()
            .await
            .context("Failed to fetch RSS feed")?;

        if !response.status().is_success() {
            bail!("RSS feed request failed with status: {}", response.status());
        }

        let content = response
            .text()
            .await
            .context("Failed to read RSS feed content")?;

        // Simple RSS parsing - look for version patterns in the content
        let regex =
            regex::Regex::new(&self.version_regex).context("Invalid version regex pattern")?;

        let mut versions = Vec::new();
        let mut seen_versions = std::collections::HashSet::new();

        for captures in regex.captures_iter(&content) {
            if let Some(version_match) = captures.get(1) {
                let version = version_match.as_str().to_string();
                if seen_versions.insert(version.clone()) {
                    versions.push(VersionInfo::new(&version, self.release_type.clone()));
                }
            }
        }

        // Sort versions (newest first)
        versions.sort_by(|a, b| b.cmp(a));

        // Limit to 20 most recent versions
        versions.truncate(20);

        Ok(versions)
    }
}

/// GitHub releases based version detector
#[derive(Debug, Clone)]
pub struct GitHubVersionDetector {
    pub repo_owner: String,
    pub repo_name: String,
    pub version_prefix: Option<String>,
    pub include_prereleases: bool,
    client: Client,
}

impl GitHubVersionDetector {
    pub fn new(repo_owner: String, repo_name: String, include_prereleases: bool) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("isod/0.1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            repo_owner,
            repo_name,
            version_prefix: None,
            include_prereleases,
            client,
        }
    }

    pub fn with_version_prefix(mut self, prefix: String) -> Self {
        self.version_prefix = Some(prefix);
        self
    }
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    published_at: Option<String>,
    prerelease: bool,
    draft: bool,
}

#[async_trait]
impl VersionDetector for GitHubVersionDetector {
    async fn detect_versions(&self) -> Result<Vec<VersionInfo>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases",
            self.repo_owner, self.repo_name
        );

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .context("Failed to fetch GitHub releases")?;

        if !response.status().is_success() {
            bail!(
                "GitHub API request failed with status: {}",
                response.status()
            );
        }

        let releases: Vec<GitHubRelease> = response
            .json()
            .await
            .context("Failed to parse GitHub releases JSON")?;

        let mut versions = Vec::new();

        for release in releases {
            if release.draft {
                continue;
            }

            if release.prerelease && !self.include_prereleases {
                continue;
            }

            let mut version = release.tag_name.clone();

            // Remove version prefix if specified
            if let Some(prefix) = &self.version_prefix {
                if version.starts_with(prefix) {
                    version = version[prefix.len()..].to_string();
                }
            }

            // Remove common prefixes
            if version.starts_with('v') {
                version = version[1..].to_string();
            }

            let release_type = if release.prerelease {
                if version.contains("rc") {
                    ReleaseType::RC
                } else if version.contains("beta") {
                    ReleaseType::Beta
                } else if version.contains("alpha") {
                    ReleaseType::Alpha
                } else {
                    ReleaseType::Beta
                }
            } else {
                ReleaseType::Stable
            };

            let mut version_info = VersionInfo::new(&version, release_type);

            if let Some(published_at) = release.published_at {
                // Extract date from ISO format (2023-04-18T10:30:00Z)
                if let Some(date_part) = published_at.split('T').next() {
                    version_info = version_info.with_release_date(date_part);
                }
            }

            versions.push(version_info);
        }

        Ok(versions)
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
    client: Client,
}

impl WebScrapingDetector {
    pub fn new(base_url: String, version_selector: String, version_regex: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("isod/0.1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            base_url,
            version_selector,
            version_regex,
            date_selector: None,
            date_format: None,
            client,
        }
    }
}

#[async_trait]
impl VersionDetector for WebScrapingDetector {
    async fn detect_versions(&self) -> Result<Vec<VersionInfo>> {
        let response = self
            .client
            .get(&self.base_url)
            .send()
            .await
            .context("Failed to fetch web page")?;

        if !response.status().is_success() {
            bail!("Web request failed with status: {}", response.status());
        }

        let content = response
            .text()
            .await
            .context("Failed to read web page content")?;

        // Simple regex-based extraction
        let regex =
            regex::Regex::new(&self.version_regex).context("Invalid version regex pattern")?;

        let mut versions = Vec::new();
        let mut seen_versions = std::collections::HashSet::new();

        for captures in regex.captures_iter(&content) {
            if let Some(version_match) = captures.get(1) {
                let version = version_match.as_str().to_string();
                if seen_versions.insert(version.clone()) {
                    versions.push(VersionInfo::new(&version, ReleaseType::Stable));
                }
            }
        }

        // Sort versions (newest first)
        versions.sort_by(|a, b| b.cmp(a));

        Ok(versions)
    }
}

/// API-based version detector for distributions with APIs
#[derive(Debug, Clone)]
pub struct ApiVersionDetector {
    pub api_url: String,
    pub auth_header: Option<String>,
    pub version_json_path: String, // JSONPath to version field
    pub date_json_path: Option<String>,
    client: Client,
}

impl ApiVersionDetector {
    pub fn new(api_url: String, version_json_path: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("isod/0.1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            api_url,
            auth_header: None,
            version_json_path,
            date_json_path: None,
            client,
        }
    }
}

#[async_trait]
impl VersionDetector for ApiVersionDetector {
    async fn detect_versions(&self) -> Result<Vec<VersionInfo>> {
        let mut request = self.client.get(&self.api_url);

        if let Some(auth) = &self.auth_header {
            request = request.header("Authorization", auth);
        }

        let response = request.send().await.context("Failed to fetch API data")?;

        if !response.status().is_success() {
            bail!("API request failed with status: {}", response.status());
        }

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse API JSON response")?;

        // Simple JSONPath-like extraction
        let mut versions = Vec::new();

        // For now, implement basic JSON traversal
        if let Some(array) = json.as_array() {
            for item in array {
                if let Some(version_str) = self.extract_json_value(item, &self.version_json_path) {
                    versions.push(VersionInfo::new(&version_str, ReleaseType::Stable));
                }
            }
        }

        Ok(versions)
    }
}

impl ApiVersionDetector {
    fn extract_json_value(&self, json: &serde_json::Value, path: &str) -> Option<String> {
        // Simple implementation - just handle basic object property access
        if let Some(field_name) = path.strip_prefix("$.") {
            json.get(field_name)?.as_str().map(|s| s.to_string())
        } else {
            None
        }
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
        Box::new(GitHubVersionDetector::new(
            owner.to_string(),
            repo.to_string(),
            include_prereleases,
        ))
    }

    /// Create an RSS feed detector
    pub fn rss_feed(
        feed_url: &str,
        version_regex: &str,
        release_type: ReleaseType,
    ) -> Box<dyn VersionDetector> {
        Box::new(FeedVersionDetector::new(
            feed_url.to_string(),
            version_regex.to_string(),
            release_type,
        ))
    }

    /// Create a web scraping detector
    pub fn web_scraper(
        base_url: &str,
        version_selector: &str,
        version_regex: &str,
    ) -> Box<dyn VersionDetector> {
        Box::new(WebScrapingDetector::new(
            base_url.to_string(),
            version_selector.to_string(),
            version_regex.to_string(),
        ))
    }

    /// Create an API detector
    pub fn api(api_url: &str, version_json_path: &str) -> Box<dyn VersionDetector> {
        Box::new(ApiVersionDetector::new(
            api_url.to_string(),
            version_json_path.to_string(),
        ))
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
