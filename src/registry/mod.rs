pub mod distros;
pub mod sources;
pub mod version_detection;

use anyhow::{Context, Result, bail};
use console::{Term, style};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

pub use sources::DownloadSource;
pub use version_detection::{ReleaseType, VersionDetector, VersionInfo};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsoInfo {
    pub distro: String,
    pub version: String,
    pub architecture: String,
    pub variant: Option<String>,
    pub filename: String,
    pub download_sources: Vec<DownloadSource>,
    pub checksum: Option<String>,
    pub checksum_type: Option<String>,
    pub release_date: Option<String>,
    pub size_bytes: Option<u64>,
    pub release_type: ReleaseType,
}

#[derive(Debug)]
pub struct DistroDefinition {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub homepage: String,
    pub supported_architectures: Vec<String>,
    pub supported_variants: Vec<String>,
    pub version_detector: Box<dyn VersionDetector>,
    pub download_sources: Vec<DownloadSource>,
    pub filename_pattern: String,
    pub default_variant: Option<String>,
    pub checksum_urls: Vec<String>,
}

pub struct IsoRegistry {
    distros: HashMap<String, DistroDefinition>,
    custom_distros: HashMap<String, DistroDefinition>,
    http_client: Client,
}

impl IsoRegistry {
    pub fn new() -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("isod/0.1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        let mut registry = Self {
            distros: HashMap::new(),
            custom_distros: HashMap::new(),
            http_client,
        };

        // Load built-in distro definitions
        registry.load_builtin_distros();
        registry
    }

    /// Load all built-in distro definitions
    fn load_builtin_distros(&mut self) {
        let term = Term::stderr();
        let _ = term.write_line(&format!(
            "{} Loading built-in distro definitions...",
            style("📦").cyan()
        ));

        // Load each distro definition
        if let Ok(ubuntu) = distros::ubuntu::create_definition() {
            self.distros.insert("ubuntu".to_string(), ubuntu);
        }

        if let Ok(fedora) = distros::fedora::create_definition() {
            self.distros.insert("fedora".to_string(), fedora);
        }

        if let Ok(debian) = distros::debian::create_definition() {
            self.distros.insert("debian".to_string(), debian);
        }

        if let Ok(arch) = distros::arch::create_definition() {
            self.distros.insert("arch".to_string(), arch);
        }

        let _ = term.write_line(&format!(
            "{} Loaded {} built-in distro definitions",
            style("✅").green(),
            style(self.distros.len()).green().bold()
        ));
    }

    /// Get all available distros (built-in + custom)
    pub fn get_all_distros(&self) -> Vec<&str> {
        let mut distros: Vec<&str> = self.distros.keys().map(|s| s.as_str()).collect();
        distros.extend(self.custom_distros.keys().map(|s| s.as_str()));
        distros.sort();
        distros
    }

    /// Get a specific distro definition
    pub fn get_distro(&self, name: &str) -> Option<&DistroDefinition> {
        self.distros
            .get(name)
            .or_else(|| self.custom_distros.get(name))
    }

    /// Check if a distro is supported
    pub fn is_supported(&self, name: &str) -> bool {
        self.distros.contains_key(name) || self.custom_distros.contains_key(name)
    }

    /// Get available versions for a distro
    pub async fn get_available_versions(&self, distro: &str) -> Result<Vec<VersionInfo>> {
        let definition = self
            .get_distro(distro)
            .with_context(|| format!("Distro '{}' not found in registry", distro))?;

        definition
            .version_detector
            .detect_versions()
            .await
            .with_context(|| format!("Failed to detect versions for {}", distro))
    }

    /// Get latest version for a distro
    pub async fn get_latest_version(&self, distro: &str) -> Result<VersionInfo> {
        let versions = self.get_available_versions(distro).await?;

        // Try to find stable/LTS versions first
        let stable_version = versions
            .iter()
            .filter(|v| matches!(v.release_type, ReleaseType::Stable | ReleaseType::LTS))
            .max_by(|a, b| a.cmp(b))
            .cloned();

        if let Some(version) = stable_version {
            return Ok(version);
        }

        // Fallback to any version if no stable versions found
        versions
            .into_iter()
            .max_by(|a, b| a.cmp(b))
            .context("No versions found")
    }

    /// Get ISO information for a specific distro/version/arch/variant combination
    pub async fn get_iso_info(
        &self,
        distro: &str,
        version: Option<&str>,
        architecture: Option<&str>,
        variant: Option<&str>,
    ) -> Result<IsoInfo> {
        let definition = self
            .get_distro(distro)
            .with_context(|| format!("Distro '{}' not found", distro))?;

        // Use latest version if not specified
        let version_info = if let Some(v) = version {
            // Try to find the specific version
            let versions = self.get_available_versions(distro).await?;
            versions
                .into_iter()
                .find(|vi| vi.version == v)
                .with_context(|| format!("Version '{}' not found for {}", v, distro))?
        } else {
            self.get_latest_version(distro).await?
        };

        // Use default architecture if not specified
        let arch = architecture.unwrap_or_else(|| {
            definition
                .supported_architectures
                .first()
                .map(|s| s.as_str())
                .unwrap_or("amd64")
        });

        // Validate architecture
        if !definition
            .supported_architectures
            .contains(&arch.to_string())
        {
            bail!("Architecture '{}' not supported for {}", arch, distro);
        }

        // Use default variant if not specified
        let variant_str = variant.or_else(|| definition.default_variant.as_deref());

        // Validate variant if specified
        if let Some(v) = variant_str {
            if !definition.supported_variants.contains(&v.to_string()) {
                bail!("Variant '{}' not supported for {}", v, distro);
            }
        }

        // Generate filename using pattern
        let filename =
            self.generate_filename(definition, &version_info.version, arch, variant_str)?;

        // Get download sources with version/arch specific URLs
        let download_sources = self
            .resolve_download_sources(
                definition,
                &version_info.version,
                arch,
                variant_str,
                &filename,
            )
            .await?;

        Ok(IsoInfo {
            distro: distro.to_string(),
            version: version_info.version,
            architecture: arch.to_string(),
            variant: variant_str.map(|s| s.to_string()),
            filename,
            download_sources,
            checksum: None, // Will be fetched when needed
            checksum_type: Some("sha256".to_string()),
            release_date: version_info.release_date,
            size_bytes: None, // Will be determined during download
            release_type: version_info.release_type,
        })
    }

    /// Add a custom distro definition
    pub fn add_custom_distro(&mut self, definition: DistroDefinition) {
        let name = definition.name.clone();
        self.custom_distros.insert(name.clone(), definition);

        let term = Term::stderr();
        let _ = term.write_line(&format!(
            "{} Added custom distro definition: {}",
            style("✅").green(),
            style(&name).cyan()
        ));
    }

    /// Remove a custom distro definition
    pub fn remove_custom_distro(&mut self, name: &str) -> bool {
        if self.custom_distros.remove(name).is_some() {
            let term = Term::stderr();
            let _ = term.write_line(&format!(
                "{} Removed custom distro definition: {}",
                style("🗑️").yellow(),
                style(name).cyan()
            ));
            true
        } else {
            false
        }
    }

    /// Search for distros by name or description
    pub fn search_distros(&self, query: &str) -> Vec<&str> {
        let query_lower = query.to_lowercase();
        let mut matches = Vec::new();

        for (name, definition) in &self.distros {
            if name.contains(&query_lower)
                || definition
                    .display_name
                    .to_lowercase()
                    .contains(&query_lower)
                || definition.description.to_lowercase().contains(&query_lower)
            {
                matches.push(name.as_str());
            }
        }

        for (name, definition) in &self.custom_distros {
            if name.contains(&query_lower)
                || definition
                    .display_name
                    .to_lowercase()
                    .contains(&query_lower)
                || definition.description.to_lowercase().contains(&query_lower)
            {
                matches.push(name.as_str());
            }
        }

        matches.sort();
        matches
    }

    /// Generate filename using the distro's pattern
    fn generate_filename(
        &self,
        definition: &DistroDefinition,
        version: &str,
        architecture: &str,
        variant: Option<&str>,
    ) -> Result<String> {
        let mut filename = definition.filename_pattern.clone();

        // Replace placeholders
        filename = filename.replace("{distro}", &definition.name);
        filename = filename.replace("{version}", version);
        filename = filename.replace("{arch}", architecture);

        if let Some(variant) = variant {
            filename = filename.replace("{variant}", variant);
        } else {
            // Remove variant placeholder if no variant specified
            filename = filename.replace("-{variant}", "");
            filename = filename.replace("_{variant}", "");
            filename = filename.replace("{variant}-", "");
            filename = filename.replace("{variant}_", "");
            filename = filename.replace("{variant}", "");
        }

        Ok(filename)
    }

    /// Resolve download sources with actual URLs
    async fn resolve_download_sources(
        &self,
        definition: &DistroDefinition,
        version: &str,
        architecture: &str,
        variant: Option<&str>,
        filename: &str,
    ) -> Result<Vec<DownloadSource>> {
        let mut resolved_sources = Vec::new();

        for source in &definition.download_sources {
            let mut resolved_source = source.clone();

            // Replace placeholders in URLs
            if let Some(url) = &mut resolved_source.url {
                *url = url.replace("{version}", version);
                *url = url.replace("{arch}", architecture);
                *url = url.replace("{filename}", filename);

                if let Some(variant) = variant {
                    *url = url.replace("{variant}", variant);
                }
            }

            resolved_sources.push(resolved_source);
        }

        Ok(resolved_sources)
    }

    /// Get checksum for an ISO with actual HTTP fetching
    pub async fn get_checksum(&self, iso_info: &IsoInfo) -> Result<Option<String>> {
        let definition = self
            .get_distro(&iso_info.distro)
            .context("Distro definition not found")?;

        for checksum_url_pattern in &definition.checksum_urls {
            let mut checksum_url = checksum_url_pattern.clone();

            // Replace placeholders
            checksum_url = checksum_url.replace("{version}", &iso_info.version);
            checksum_url = checksum_url.replace("{arch}", &iso_info.architecture);
            checksum_url = checksum_url.replace("{filename}", &iso_info.filename);

            if let Some(variant) = &iso_info.variant {
                checksum_url = checksum_url.replace("{variant}", variant);
            }

            // Try to fetch checksum from this URL
            if let Ok(checksum) = self.fetch_checksum(&checksum_url, &iso_info.filename).await {
                return Ok(Some(checksum));
            }
        }

        Ok(None)
    }

    /// Fetch checksum from a URL with actual HTTP implementation
    async fn fetch_checksum(&self, url: &str, filename: &str) -> Result<String> {
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch checksum from: {}", url))?;

        if !response.status().is_success() {
            bail!("HTTP request failed with status: {}", response.status());
        }

        let content = response
            .text()
            .await
            .context("Failed to read checksum response as text")?;

        // Parse checksum file formats (SHA256SUMS, MD5SUMS, etc.)
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Handle different checksum file formats:
            // Format 1: "checksum filename" (space separated)
            // Format 2: "checksum *filename" (binary mode indicator)
            // Format 3: "checksum  filename" (double space)

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let checksum = parts[0];
                let file_in_line = parts[1].trim_start_matches('*'); // Remove binary mode indicator

                // Check if this line contains our filename
                if file_in_line == filename || file_in_line.ends_with(filename) {
                    // Validate checksum format (should be hex)
                    if checksum.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Ok(checksum.to_lowercase());
                    }
                }
            }

            // Alternative format: "filename: checksum"
            if let Some(colon_pos) = line.find(':') {
                let (file_part, checksum_part) = line.split_at(colon_pos);
                let file_part = file_part.trim();
                let checksum = checksum_part[1..].trim(); // Remove the colon

                if (file_part == filename || file_part.ends_with(filename))
                    && checksum.chars().all(|c| c.is_ascii_hexdigit())
                {
                    return Ok(checksum.to_lowercase());
                }
            }
        }

        bail!("Checksum for '{}' not found in checksum file", filename);
    }
}

impl Default for IsoRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for IsoInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}-{}", self.distro, self.version, self.architecture)?;
        if let Some(variant) = &self.variant {
            write!(f, "-{}", variant)?;
        }
        write!(f, ".iso")
    }
}
