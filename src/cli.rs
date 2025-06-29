use clap::{Parser, Subcommand};
use std::ffi::OsString;

#[derive(Parser)]
#[command(name = "isod")]
#[command(about = "A tool to manage bootable ISOs on Ventoy USB keys")]
#[command(long_about = "
isod is a command-line tool for automatically downloading, updating, and managing
bootable ISO files on Ventoy USB drives. It supports multiple Linux distributions
with automatic version detection, torrent/mirror downloads, and USB synchronization.

Examples:
  isod add ubuntu                    # Add Ubuntu with defaults
  isod add fedora --variant workstation --arch x86_64
  isod update                        # Update all configured ISOs
  isod sync                          # Sync with Ventoy USB device
  isod list --installed              # Show ISOs on USB device
")]
#[command(version)]
pub struct Cli {
    /// Override config file path
    #[arg(short, long, global = true, value_name = "FILE")]
    pub config: Option<String>,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add ISO to download list
    #[command(visible_alias = "a")]
    Add {
        /// Distribution name (e.g., ubuntu, fedora, debian, arch)
        distro: String,

        /// Specific variant to download
        #[arg(short, long, value_name = "VARIANT")]
        #[arg(help = "Distribution variant (e.g., desktop, server, workstation)")]
        variant: Option<String>,

        /// Target architecture
        #[arg(short, long, value_name = "ARCH")]
        #[arg(help = "Architecture (e.g., amd64, x86_64, arm64)")]
        arch: Option<String>,

        /// Specific version to download
        #[arg(short = 'V', long, value_name = "VERSION")]
        #[arg(help = "Specific version (defaults to latest)")]
        version: Option<String>,

        /// Add all supported variants
        #[arg(long)]
        #[arg(help = "Add all supported variants for this distribution")]
        all_variants: bool,

        /// Add all supported architectures
        #[arg(long)]
        #[arg(help = "Add all supported architectures for this distribution")]
        all_archs: bool,
    },

    /// Update specific or all ISOs
    #[command(visible_alias = "u")]
    Update {
        /// Specific distro to update (updates all if not specified)
        distro: Option<OsString>,

        /// Force update even if recent version exists
        #[arg(short, long)]
        #[arg(help = "Force update even if current version is recent")]
        force: bool,

        /// Check for updates without downloading
        #[arg(long)]
        #[arg(help = "Check for updates without downloading")]
        check_only: bool,

        /// Include beta/development versions
        #[arg(long)]
        #[arg(help = "Include beta and development versions")]
        include_beta: bool,
    },

    /// List available/installed ISOs
    #[command(visible_alias = "ls")]
    List {
        /// Show only installed ISOs on USB device
        #[arg(short, long)]
        #[arg(help = "Show ISOs installed on USB device")]
        installed: bool,

        /// Show available versions for each distro
        #[arg(short, long)]
        #[arg(help = "Show available versions for each distribution")]
        versions: bool,

        /// Filter by distribution name
        #[arg(short, long, value_name = "DISTRO")]
        #[arg(help = "Filter results by distribution name")]
        distro: Option<String>,

        /// Show detailed information
        #[arg(short = 'l', long)]
        #[arg(help = "Show detailed information")]
        long: bool,
    },

    /// Remove ISO from USB device
    #[command(visible_alias = "rm")]
    Remove {
        /// Distribution name to remove
        distro: String,

        /// Specific variant to remove
        #[arg(short, long, value_name = "VARIANT")]
        #[arg(help = "Remove only specific variant")]
        variant: Option<String>,

        /// Specific version to remove
        #[arg(short = 'V', long, value_name = "VERSION")]
        #[arg(help = "Remove only specific version")]
        version: Option<String>,

        /// Remove all versions of this distro
        #[arg(long)]
        #[arg(help = "Remove all versions of this distribution")]
        all: bool,

        /// Skip confirmation prompt
        #[arg(short, long)]
        #[arg(help = "Skip confirmation prompt")]
        yes: bool,
    },

    /// Sync with USB key
    #[command(visible_alias = "s")]
    Sync {
        /// USB mount point override
        #[arg(short, long, value_name = "PATH")]
        #[arg(help = "Override USB mount point")]
        mount_point: Option<String>,

        /// Auto-select first Ventoy device found
        #[arg(short, long)]
        #[arg(help = "Automatically select first Ventoy device")]
        auto: bool,

        /// Verify checksums of existing ISOs
        #[arg(long)]
        #[arg(help = "Verify checksums of existing ISOs")]
        verify: bool,

        /// Download missing ISOs after sync
        #[arg(short, long)]
        #[arg(help = "Download missing ISOs after sync")]
        download: bool,
    },

    /// Manage configuration
    #[command(visible_alias = "cfg")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Clean old versions
    #[command(visible_alias = "cleanup")]
    Clean {
        /// Keep only the latest N versions per distribution
        #[arg(short, long, default_value = "2", value_name = "N")]
        #[arg(help = "Number of versions to keep per distribution")]
        keep: u32,

        /// Dry run - show what would be deleted without deleting
        #[arg(short, long)]
        #[arg(help = "Show what would be deleted without actually deleting")]
        dry_run: bool,

        /// Minimum age in days before considering for cleanup
        #[arg(long, default_value = "30", value_name = "DAYS")]
        #[arg(help = "Minimum age in days before considering for cleanup")]
        min_age: u32,

        /// Clean only specific distribution
        #[arg(long, value_name = "DISTRO")]
        #[arg(help = "Clean only specific distribution")]
        distro: Option<String>,

        /// Also clean downloaded files in cache
        #[arg(long)]
        #[arg(help = "Also clean downloaded files in cache directory")]
        cache: bool,
    },

    /// Download ISOs without USB operations
    #[command(visible_alias = "dl")]
    Download {
        /// Distribution name to download
        distro: String,

        /// Download directory override
        #[arg(short, long, value_name = "DIR")]
        #[arg(help = "Download to specific directory")]
        output_dir: Option<String>,

        /// Specific variant to download
        #[arg(short, long, value_name = "VARIANT")]
        variant: Option<String>,

        /// Target architecture
        #[arg(short, long, value_name = "ARCH")]
        arch: Option<String>,

        /// Specific version to download
        #[arg(short = 'V', long, value_name = "VERSION")]
        version: Option<String>,

        /// Prefer torrent downloads
        #[arg(short, long)]
        #[arg(help = "Prefer torrent downloads over HTTP")]
        torrent: bool,

        /// Maximum concurrent downloads
        #[arg(short, long, default_value = "3", value_name = "N")]
        #[arg(help = "Maximum concurrent downloads")]
        max_concurrent: u8,

        /// Verify checksum after download
        #[arg(long)]
        #[arg(help = "Verify checksum after download")]
        verify: bool,
    },

    /// Search for distributions
    Search {
        /// Search query (searches name and description)
        query: String,

        /// Show detailed information for matches
        #[arg(short, long)]
        #[arg(help = "Show detailed information for matches")]
        detailed: bool,

        /// Maximum number of results to show
        #[arg(short, long, default_value = "20", value_name = "N")]
        #[arg(help = "Maximum number of results to show")]
        limit: usize,
    },

    /// Show information about a distribution
    Info {
        /// Distribution name
        distro: String,

        /// Show available versions
        #[arg(short, long)]
        #[arg(help = "Show available versions")]
        versions: bool,

        /// Show download sources
        #[arg(short, long)]
        #[arg(help = "Show download sources")]
        sources: bool,

        /// Show supported architectures and variants
        #[arg(short, long)]
        #[arg(help = "Show supported architectures and variants")]
        details: bool,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show {
        /// Show only specific section
        #[arg(short, long, value_name = "SECTION")]
        #[arg(help = "Show only specific section (general, usb, sources, distros)")]
        section: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "toml")]
        #[arg(help = "Output format")]
        format: ConfigFormat,
    },

    /// Edit configuration file
    Edit {
        /// Editor to use (overrides $EDITOR)
        #[arg(short, long, value_name = "EDITOR")]
        #[arg(help = "Editor to use (overrides $EDITOR environment variable)")]
        editor: Option<String>,
    },

    /// Validate configuration
    #[command(visible_alias = "check")]
    Validate {
        /// Fix common issues automatically
        #[arg(short, long)]
        #[arg(help = "Fix common configuration issues automatically")]
        fix: bool,

        /// Show warnings as well as errors
        #[arg(short, long)]
        #[arg(help = "Show warnings as well as errors")]
        warnings: bool,
    },

    /// Create sample configuration
    Sample {
        /// Output file (defaults to config.sample.toml)
        #[arg(short, long, value_name = "FILE")]
        #[arg(help = "Output file path")]
        output: Option<String>,

        /// Overwrite existing file
        #[arg(short, long)]
        #[arg(help = "Overwrite existing file")]
        force: bool,
    },

    /// Set a configuration value
    Set {
        /// Configuration key (e.g., general.max_concurrent_downloads)
        key: String,

        /// Configuration value
        value: String,

        /// Value type hint
        #[arg(short, long, value_enum)]
        #[arg(help = "Value type hint for parsing")]
        value_type: Option<ValueType>,
    },

    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,

        /// Output format
        #[arg(short, long, value_enum, default_value = "plain")]
        #[arg(help = "Output format")]
        format: ConfigFormat,
    },

    /// Reset configuration to defaults
    Reset {
        /// Section to reset (resets all if not specified)
        #[arg(short, long, value_name = "SECTION")]
        #[arg(help = "Section to reset (general, usb, sources, distros)")]
        section: Option<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        #[arg(help = "Skip confirmation prompt")]
        yes: bool,
    },

    /// Import configuration from file
    Import {
        /// Configuration file to import
        file: String,

        /// Merge with existing config instead of replacing
        #[arg(short, long)]
        #[arg(help = "Merge with existing configuration")]
        merge: bool,
    },

    /// Export configuration to file
    Export {
        /// Output file
        file: String,

        /// Export format
        #[arg(short, long, value_enum, default_value = "toml")]
        #[arg(help = "Export format")]
        format: ConfigFormat,

        /// Include comments and documentation
        #[arg(short, long)]
        #[arg(help = "Include comments and documentation")]
        documented: bool,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ConfigFormat {
    /// TOML format (default)
    Toml,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// Plain text (for single values)
    Plain,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ValueType {
    /// String value
    String,
    /// Integer value
    Int,
    /// Boolean value
    Bool,
    /// Array/list value
    Array,
}

impl std::fmt::Display for ConfigFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigFormat::Toml => write!(f, "toml"),
            ConfigFormat::Json => write!(f, "json"),
            ConfigFormat::Yaml => write!(f, "yaml"),
            ConfigFormat::Plain => write!(f, "plain"),
        }
    }
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueType::String => write!(f, "string"),
            ValueType::Int => write!(f, "int"),
            ValueType::Bool => write!(f, "bool"),
            ValueType::Array => write!(f, "array"),
        }
    }
}

// Helper functions for CLI validation and parsing
impl Cli {
    /// Validate CLI arguments and show helpful error messages
    pub fn validate(&self) -> Result<(), String> {
        match &self.command {
            Commands::Add { distro, .. } => {
                if distro.is_empty() {
                    return Err("Distribution name cannot be empty".to_string());
                }
            }
            Commands::Remove {
                distro,
                version,
                all,
                ..
            } => {
                if distro.is_empty() {
                    return Err("Distribution name cannot be empty".to_string());
                }
                if version.is_some() && *all {
                    return Err("Cannot specify both --version and --all".to_string());
                }
            }
            Commands::Clean { keep, min_age, .. } => {
                if *keep == 0 {
                    return Err("Keep count must be greater than 0".to_string());
                }
                if *min_age > 365 {
                    return Err("Minimum age cannot be more than 365 days".to_string());
                }
            }
            Commands::Download { max_concurrent, .. } => {
                if *max_concurrent == 0 || *max_concurrent > 10 {
                    return Err("Max concurrent downloads must be between 1 and 10".to_string());
                }
            }
            Commands::Search { limit, .. } => {
                if *limit == 0 || *limit > 100 {
                    return Err("Search limit must be between 1 and 100".to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Get the distro name from various command types
    pub fn get_distro_name(&self) -> Option<&str> {
        match &self.command {
            Commands::Add { distro, .. } => Some(distro),
            Commands::Remove { distro, .. } => Some(distro),
            Commands::Download { distro, .. } => Some(distro),
            Commands::Info { distro, .. } => Some(distro),
            Commands::List {
                distro: Some(distro),
                ..
            } => Some(distro),
            Commands::Update {
                distro: Some(distro),
                ..
            } => distro.to_str(),
            Commands::Clean {
                distro: Some(distro),
                ..
            } => Some(distro),
            _ => None,
        }
    }

    /// Check if command requires USB device
    pub fn requires_usb(&self) -> bool {
        matches!(
            self.command,
            Commands::Sync { .. }
                | Commands::Remove { .. }
                | Commands::Clean { .. }
                | Commands::List {
                    installed: true,
                    ..
                }
        )
    }

    /// Check if command modifies configuration
    pub fn modifies_config(&self) -> bool {
        matches!(
            self.command,
            Commands::Add { .. }
                | Commands::Config {
                    action: ConfigAction::Set { .. }
                        | ConfigAction::Reset { .. }
                        | ConfigAction::Import { .. }
                }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parsing() {
        // Test basic commands
        let cli = Cli::try_parse_from(["isod", "list"]).unwrap();
        assert!(matches!(cli.command, Commands::List { .. }));

        let cli = Cli::try_parse_from(["isod", "add", "ubuntu"]).unwrap();
        assert!(matches!(cli.command, Commands::Add { .. }));
        assert_eq!(cli.get_distro_name(), Some("ubuntu"));
    }

    #[test]
    fn test_add_command_options() {
        let cli = Cli::try_parse_from([
            "isod",
            "add",
            "ubuntu",
            "--variant",
            "desktop",
            "--arch",
            "amd64",
            "--version",
            "24.04",
        ])
        .unwrap();

        if let Commands::Add {
            distro,
            variant,
            arch,
            version,
            ..
        } = cli.command
        {
            assert_eq!(distro, "ubuntu");
            assert_eq!(variant, Some("desktop".to_string()));
            assert_eq!(arch, Some("amd64".to_string()));
            assert_eq!(version, Some("24.04".to_string()));
        } else {
            panic!("Expected Add command");
        }
    }

    #[test]
    fn test_config_subcommands() {
        let cli = Cli::try_parse_from(["isod", "config", "show"]).unwrap();
        if let Commands::Config { action } = cli.command {
            assert!(matches!(action, ConfigAction::Show { .. }));
        } else {
            panic!("Expected Config command");
        }
    }

    #[test]
    fn test_validation() {
        let cli = Cli::try_parse_from(["isod", "add", "ubuntu"]).unwrap();
        assert!(cli.validate().is_ok());

        let cli = Cli::try_parse_from(["isod", "clean", "--keep", "0"]).unwrap();
        assert!(cli.validate().is_err());
    }

    #[test]
    fn test_helper_methods() {
        let cli = Cli::try_parse_from(["isod", "sync"]).unwrap();
        assert!(cli.requires_usb());
        assert!(!cli.modifies_config());

        let cli = Cli::try_parse_from(["isod", "add", "ubuntu"]).unwrap();
        assert!(!cli.requires_usb());
        assert!(cli.modifies_config());
    }
}
