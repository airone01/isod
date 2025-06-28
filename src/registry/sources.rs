use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceType {
    Direct,
    Mirror,
    Torrent,
    Magnet,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourcePriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Preferred = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSource {
    pub source_type: SourceType,
    pub priority: SourcePriority,
    pub url: Option<String>,
    pub magnet_link: Option<String>,
    pub trackers: Vec<String>,
    pub region: Option<String>,
    pub description: Option<String>,
    pub verified: bool,
    pub speed_rating: Option<u8>, // 1-10 rating
}

impl DownloadSource {
    /// Create a new direct download source
    pub fn direct(url: &str, priority: SourcePriority) -> Self {
        Self {
            source_type: SourceType::Direct,
            priority,
            url: Some(url.to_string()),
            magnet_link: None,
            trackers: Vec::new(),
            region: None,
            description: None,
            verified: false,
            speed_rating: None,
        }
    }

    /// Create a new mirror source
    pub fn mirror(url: &str, priority: SourcePriority, region: Option<&str>) -> Self {
        Self {
            source_type: SourceType::Mirror,
            priority,
            url: Some(url.to_string()),
            magnet_link: None,
            trackers: Vec::new(),
            region: region.map(|s| s.to_string()),
            description: None,
            verified: false,
            speed_rating: None,
        }
    }

    /// Create a new torrent source
    pub fn torrent(torrent_url: &str, priority: SourcePriority) -> Self {
        Self {
            source_type: SourceType::Torrent,
            priority,
            url: Some(torrent_url.to_string()),
            magnet_link: None,
            trackers: Vec::new(),
            region: None,
            description: None,
            verified: false,
            speed_rating: None,
        }
    }

    /// Create a new magnet link source
    pub fn magnet(magnet_link: &str, priority: SourcePriority, trackers: Vec<String>) -> Self {
        Self {
            source_type: SourceType::Magnet,
            priority,
            url: None,
            magnet_link: Some(magnet_link.to_string()),
            trackers,
            region: None,
            description: None,
            verified: false,
            speed_rating: None,
        }
    }

    /// Set description for the source
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Mark source as verified
    pub fn verified(mut self) -> Self {
        self.verified = true;
        self
    }

    /// Set speed rating (1-10)
    pub fn with_speed_rating(mut self, rating: u8) -> Self {
        self.speed_rating = Some(rating.clamp(1, 10));
        self
    }

    /// Set region for the source
    pub fn with_region(mut self, region: &str) -> Self {
        self.region = Some(region.to_string());
        self
    }

    /// Get the primary URL for this source
    pub fn get_url(&self) -> Option<&str> {
        self.url.as_deref().or(self.magnet_link.as_deref())
    }

    /// Check if this source is usable (has required fields)
    pub fn is_usable(&self) -> bool {
        match self.source_type {
            SourceType::Direct | SourceType::Mirror | SourceType::Torrent => self.url.is_some(),
            SourceType::Magnet => self.magnet_link.is_some(),
        }
    }

    /// Get a score for source selection (higher is better)
    pub fn get_selection_score(&self) -> u32 {
        let mut score = self.priority as u32 * 1000;

        // Add speed rating bonus
        if let Some(speed) = self.speed_rating {
            score += speed as u32 * 100;
        }

        // Verified sources get bonus
        if self.verified {
            score += 500;
        }

        // Prefer direct sources over mirrors for reliability
        match self.source_type {
            SourceType::Direct => score += 200,
            SourceType::Torrent => score += 150,
            SourceType::Magnet => score += 100,
            SourceType::Mirror => score += 50,
        }

        score
    }
}

impl PartialEq for DownloadSource {
    fn eq(&self, other: &Self) -> bool {
        self.get_url() == other.get_url() && self.source_type == other.source_type
    }
}

impl Eq for DownloadSource {}

impl PartialOrd for DownloadSource {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DownloadSource {
    fn cmp(&self, other: &Self) -> Ordering {
        // Sort by selection score (descending - best first)
        other.get_selection_score().cmp(&self.get_selection_score())
    }
}

impl fmt::Display for SourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceType::Direct => write!(f, "Direct"),
            SourceType::Mirror => write!(f, "Mirror"),
            SourceType::Torrent => write!(f, "Torrent"),
            SourceType::Magnet => write!(f, "Magnet"),
        }
    }
}

impl fmt::Display for SourcePriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourcePriority::Low => write!(f, "Low"),
            SourcePriority::Medium => write!(f, "Medium"),
            SourcePriority::High => write!(f, "High"),
            SourcePriority::Preferred => write!(f, "Preferred"),
        }
    }
}

impl fmt::Display for DownloadSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.source_type, self.priority)?;
        if let Some(region) = &self.region {
            write!(f, " [{}]", region)?;
        }
        if let Some(desc) = &self.description {
            write!(f, " - {}", desc)?;
        }
        Ok(())
    }
}

/// Collection of download sources with management methods
#[derive(Debug, Clone)]
pub struct SourceCollection {
    sources: Vec<DownloadSource>,
}

impl SourceCollection {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    pub fn from_sources(sources: Vec<DownloadSource>) -> Self {
        let mut collection = Self { sources };
        collection.sort();
        collection
    }

    /// Add a source to the collection
    pub fn add_source(&mut self, source: DownloadSource) {
        if source.is_usable() {
            self.sources.push(source);
            self.sort();
        }
    }

    /// Get sources sorted by preference
    pub fn get_sorted_sources(&self) -> &[DownloadSource] {
        &self.sources
    }

    /// Get sources of a specific type
    pub fn get_sources_by_type(&self, source_type: SourceType) -> Vec<&DownloadSource> {
        self.sources
            .iter()
            .filter(|s| s.source_type == source_type)
            .collect()
    }

    /// Get sources by priority
    pub fn get_sources_by_priority(&self, priority: SourcePriority) -> Vec<&DownloadSource> {
        self.sources
            .iter()
            .filter(|s| s.priority == priority)
            .collect()
    }

    /// Get verified sources only
    pub fn get_verified_sources(&self) -> Vec<&DownloadSource> {
        self.sources.iter().filter(|s| s.verified).collect()
    }

    /// Get sources by region preference
    pub fn get_sources_by_region(&self, preferred_region: &str) -> Vec<&DownloadSource> {
        let mut region_sources: Vec<&DownloadSource> = self
            .sources
            .iter()
            .filter(|s| s.region.as_deref() == Some(preferred_region))
            .collect();

        // Add sources without specific region as fallback
        region_sources.extend(self.sources.iter().filter(|s| s.region.is_none()));

        region_sources
    }

    /// Get the best source based on selection criteria
    pub fn get_best_source(&self) -> Option<&DownloadSource> {
        self.sources.first()
    }

    /// Get best sources for different download methods
    pub fn get_best_sources_by_method(&self) -> BestSources {
        BestSources {
            direct: self
                .get_sources_by_type(SourceType::Direct)
                .first()
                .copied(),
            mirror: self
                .get_sources_by_type(SourceType::Mirror)
                .first()
                .copied(),
            torrent: self
                .get_sources_by_type(SourceType::Torrent)
                .first()
                .copied(),
            magnet: self
                .get_sources_by_type(SourceType::Magnet)
                .first()
                .copied(),
        }
    }

    /// Filter sources by minimum speed rating
    pub fn filter_by_min_speed(&self, min_speed: u8) -> Vec<&DownloadSource> {
        self.sources
            .iter()
            .filter(|s| s.speed_rating.unwrap_or(0) >= min_speed)
            .collect()
    }

    /// Remove sources that match a predicate
    pub fn remove_sources<F>(&mut self, predicate: F)
    where
        F: Fn(&DownloadSource) -> bool,
    {
        self.sources.retain(|s| !predicate(s));
    }

    /// Sort sources by preference (best first)
    fn sort(&mut self) {
        self.sources.sort();
    }

    /// Check if collection is empty
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Get number of sources
    pub fn len(&self) -> usize {
        self.sources.len()
    }
}

impl Default for SourceCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<DownloadSource>> for SourceCollection {
    fn from(sources: Vec<DownloadSource>) -> Self {
        Self::from_sources(sources)
    }
}

/// Structure containing the best source for each download method
#[derive(Debug)]
pub struct BestSources<'a> {
    pub direct: Option<&'a DownloadSource>,
    pub mirror: Option<&'a DownloadSource>,
    pub torrent: Option<&'a DownloadSource>,
    pub magnet: Option<&'a DownloadSource>,
}

impl<'a> BestSources<'a> {
    /// Get the overall best source across all methods
    pub fn get_overall_best(&self) -> Option<&'a DownloadSource> {
        let candidates = [self.direct, self.mirror, self.torrent, self.magnet];

        candidates
            .iter()
            .filter_map(|&opt| opt)
            .max_by_key(|source| source.get_selection_score())
    }

    /// Get sources in order of preference for trying
    pub fn get_ordered_sources(&self) -> Vec<&'a DownloadSource> {
        let mut sources = Vec::new();

        // Add in order of general preference
        if let Some(direct) = self.direct {
            sources.push(direct);
        }
        if let Some(torrent) = self.torrent {
            sources.push(torrent);
        }
        if let Some(magnet) = self.magnet {
            sources.push(magnet);
        }
        if let Some(mirror) = self.mirror {
            sources.push(mirror);
        }

        // Sort by actual score
        sources.sort_by_key(|source| std::cmp::Reverse(source.get_selection_score()));
        sources
    }
}
