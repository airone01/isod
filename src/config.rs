use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Config {
    #[serde(default)]
    pub(crate) general: GeneralConfig,
    #[serde(default)]
    pub(crate) usb: UsbConfig,
    #[serde(default)]
    pub(crate) sources: SourcesConfig,
    #[serde(default)]
    pub(crate) distros: HashMap<String, DistroConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GeneralConfig {
    #[serde(default = "default_max_concurrent_downloads")]
    pub(crate) max_concurrent_downloads: u8,
    #[serde(default = "default_prefer_torrents")]
    pub(crate) prefer_torrents: bool,
    #[serde(default = "default_auto_cleanup")]
    pub(crate) auto_cleanup_old_versions: bool,
    #[serde(default = "default_check_interval_days")]
    pub(crate) check_interval_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UsbConfig {
    #[serde(default)]
    pub(crate) mount_point: Option<String>,
    #[serde(default = "default_iso_path")]
    pub(crate) iso_path: String,
    #[serde(default = "default_metadata_file")]
    pub(crate) metadata_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SourcesConfig {
    #[serde(default = "default_enable_mirrors")]
    pub(crate) enable_mirrors: bool,
    #[serde(default)]
    pub(crate) custom_mirrors: Vec<String>,
    #[serde(default = "default_mirror_timeout_secs")]
    pub(crate) mirror_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DistroConfig {
    #[serde(default)]
    pub(crate) variants: Vec<String>,
    #[serde(default)]
    pub(crate) architectures: Vec<String>,
    #[serde(default = "default_check_interval_days")]
    pub(crate) check_interval_days: u32,
    #[serde(default = "default_enabled")]
    pub(crate) enabled: bool,
}

// Default value functions
fn default_max_concurrent_downloads() -> u8 {
    3
}
fn default_prefer_torrents() -> bool {
    true
}
fn default_auto_cleanup() -> bool {
    true
}
fn default_check_interval_days() -> u32 {
    7
}
fn default_iso_path() -> String {
    "iso".to_string()
}
fn default_metadata_file() -> String {
    "isod/metadata.toml".to_string()
}
fn default_enable_mirrors() -> bool {
    true
}
fn default_mirror_timeout_secs() -> u64 {
    30
}
fn default_enabled() -> bool {
    true
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            max_concurrent_downloads: default_max_concurrent_downloads(),
            prefer_torrents: default_prefer_torrents(),
            auto_cleanup_old_versions: default_auto_cleanup(),
            check_interval_days: default_check_interval_days(),
        }
    }
}

impl Default for UsbConfig {
    fn default() -> Self {
        Self {
            mount_point: None,
            iso_path: default_iso_path(),
            metadata_file: default_metadata_file(),
        }
    }
}

impl Default for SourcesConfig {
    fn default() -> Self {
        Self {
            enable_mirrors: default_enable_mirrors(),
            custom_mirrors: Vec::new(),
            mirror_timeout_secs: default_mirror_timeout_secs(),
        }
    }
}

impl Default for DistroConfig {
    fn default() -> Self {
        Self {
            variants: Vec::new(),
            architectures: Vec::new(),
            check_interval_days: default_check_interval_days(),
            enabled: default_enabled(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut distros = HashMap::new();

        // Add some default distro configurations
        distros.insert(
            "ubuntu".to_string(),
            DistroConfig {
                variants: vec!["desktop".to_string(), "server".to_string()],
                architectures: vec!["amd64".to_string()],
                ..Default::default()
            },
        );

        distros.insert(
            "fedora".to_string(),
            DistroConfig {
                variants: vec!["workstation".to_string(), "server".to_string()],
                architectures: vec!["x86_64".to_string()],
                ..Default::default()
            },
        );

        Self {
            general: GeneralConfig::default(),
            usb: UsbConfig::default(),
            sources: SourcesConfig::default(),
            distros,
        }
    }
}

pub(crate) struct ConfigManager {
    config_dir: PathBuf,
    config_file: PathBuf,
    config: Config,
}

impl ConfigManager {
    /// Create a new ConfigManager and load existing config or create default
    pub(crate) fn new() -> Result<Self> {
        let project_dirs =
            ProjectDirs::from("", "", "isod").context("Failed to determine config directory")?;

        let config_dir = project_dirs.config_dir().to_path_buf();
        let config_file = config_dir.join("config.toml");

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .with_context(|| format!("Failed to create config directory: {:?}", config_dir))?;
        }

        let config = if config_file.exists() {
            Self::load_config(&config_file)?
        } else {
            let default_config = Config::default();
            Self::save_config(&config_file, &default_config)?;
            default_config
        };

        Ok(Self {
            config_dir,
            config_file,
            config,
        })
    }

    /// Get a reference to the current config
    pub(crate) fn config(&self) -> &Config {
        &self.config
    }

    /// Get a mutable reference to the current config
    pub(crate) fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Save the current config to disk
    pub(crate) fn save(&self) -> Result<()> {
        Self::save_config(&self.config_file, &self.config)
    }

    /// Reload config from disk
    pub(crate) fn reload(&mut self) -> Result<()> {
        self.config = Self::load_config(&self.config_file)?;
        Ok(())
    }

    /// Get the config directory path
    pub(crate) fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Get the config file path
    pub(crate) fn config_file(&self) -> &Path {
        &self.config_file
    }

    /// Add or update a distro configuration
    pub(crate) fn set_distro_config(&mut self, distro: String, config: DistroConfig) {
        self.config.distros.insert(distro, config);
    }

    /// Remove a distro configuration
    pub(crate) fn remove_distro_config(&mut self, distro: &str) -> Option<DistroConfig> {
        self.config.distros.remove(distro)
    }

    /// Get a distro configuration
    pub(crate) fn get_distro_config(&self, distro: &str) -> Option<&DistroConfig> {
        self.config.distros.get(distro)
    }

    /// Load config from file
    fn load_config(config_file: &Path) -> Result<Config> {
        let content = fs::read_to_string(config_file)
            .with_context(|| format!("Failed to read config file: {:?}", config_file))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {:?}", config_file))?;

        Ok(config)
    }

    /// Save config to file
    fn save_config(config_file: &Path, config: &Config) -> Result<()> {
        let content = toml::to_string_pretty(config).context("Failed to serialize config")?;

        fs::write(config_file, content)
            .with_context(|| format!("Failed to write config file: {:?}", config_file))?;

        Ok(())
    }

    /// Create a sample config file for user reference
    pub(crate) fn create_sample_config(&self) -> Result<PathBuf> {
        let sample_file = self.config_dir.join("config.sample.toml");
        let sample_config = Config::default();
        Self::save_config(&sample_file, &sample_config)?;
        Ok(sample_file)
    }

    /// Validate the current configuration
    pub(crate) fn validate(&self) -> Result<()> {
        // Validate general config
        if self.config.general.max_concurrent_downloads == 0 {
            anyhow::bail!("max_concurrent_downloads must be greater than 0");
        }

        if self.config.general.check_interval_days == 0 {
            anyhow::bail!("check_interval_days must be greater than 0");
        }

        // Validate USB config
        if self.config.usb.iso_path.is_empty() {
            anyhow::bail!("iso_path cannot be empty");
        }

        // Validate distro configs
        for (name, distro_config) in &self.config.distros {
            if distro_config.check_interval_days == 0 {
                anyhow::bail!(
                    "check_interval_days for distro '{}' must be greater than 0",
                    name
                );
            }
        }

        Ok(())
    }
}
