mod cli;
mod handlers;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use console::{Term, style};
use isod::usb::UsbManager;
use isod::{ConfigManager, IsoRegistry};
use std::process;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    // Validate CLI arguments first
    if let Err(e) = args.validate() {
        let term = Term::stderr();
        term.write_line(&format!("{} {}", style("Error:").red().bold(), e))?;
        process::exit(1);
    }

    // Initialize systems
    let mut config_manager = ConfigManager::new()?;
    let mut usb_manager = UsbManager::new();
    let iso_registry = IsoRegistry::new();

    // Validate config on startup (unless we're about to fix it)
    let skip_config_validation = handlers::should_skip_config_validation(&args.command);

    if !skip_config_validation {
        if let Err(e) = config_manager.validate() {
            let term = Term::stderr();
            term.write_line(&format!(
                "{} Configuration validation failed: {}",
                style("Error:").red().bold(),
                e
            ))?;
            term.write_line(&format!(
                "{} Run 'isod config validate --fix' to automatically fix common issues",
                style("Hint:").cyan()
            ))?;
            term.write_line(&format!(
                "{} Or run 'isod config validate' for detailed validation report",
                style("Hint:").cyan()
            ))?;
            process::exit(1);
        }
    }

    // Handle commands
    match args.command {
        Commands::Add {
            distro,
            variant,
            arch,
            version,
            all_variants,
            all_archs,
        } => {
            handlers::handle_add(
                &mut config_manager,
                &iso_registry,
                distro,
                variant,
                arch,
                version,
                all_variants,
                all_archs,
            )
            .await?;
        }
        Commands::Update {
            distro,
            force,
            check_only,
            include_beta,
        } => {
            handlers::handle_update(
                &config_manager,
                &iso_registry,
                distro,
                force,
                check_only,
                include_beta,
            )
            .await?;
        }
        Commands::List {
            installed,
            versions,
            distro,
            long,
        } => {
            handlers::handle_list(
                &config_manager,
                &iso_registry,
                &usb_manager,
                installed,
                versions,
                distro,
                long,
            )
            .await?;
        }
        Commands::Remove {
            distro,
            variant,
            version,
            all,
            yes,
        } => {
            handlers::handle_remove(
                &config_manager,
                &usb_manager,
                distro,
                variant,
                version,
                all,
                yes,
            )
            .await?;
        }
        Commands::Sync {
            mount_point,
            auto,
            verify,
            download,
        } => {
            handlers::handle_sync(
                &config_manager,
                &mut usb_manager,
                mount_point,
                auto,
                verify,
                download,
            )
            .await?;
        }
        Commands::Config { action } => {
            handlers::handle_config(&mut config_manager, action).await?;
        }
        Commands::Clean {
            keep,
            dry_run,
            min_age,
            distro,
            cache,
        } => {
            handlers::handle_clean(
                &config_manager,
                &usb_manager,
                keep,
                dry_run,
                min_age,
                distro,
                cache,
            )
            .await?;
        }
        Commands::Download {
            distro,
            output_dir,
            variant,
            arch,
            version,
            torrent,
            max_concurrent,
            verify,
        } => {
            handlers::handle_download(
                &iso_registry,
                distro,
                output_dir,
                variant,
                arch,
                version,
                torrent,
                max_concurrent,
                verify,
            )
            .await?;
        }
        Commands::Search {
            query,
            detailed,
            limit,
        } => {
            handlers::handle_search(&iso_registry, query, detailed, limit).await?;
        }
        Commands::Info {
            distro,
            versions,
            sources,
            details,
        } => {
            handlers::handle_info(&iso_registry, distro, versions, sources, details).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_manager_initialization() {
        // Test that config manager can be created
        let result = ConfigManager::new();
        assert!(
            result.is_ok(),
            "Config manager should initialize successfully"
        );
    }

    #[tokio::test]
    async fn test_iso_registry_initialization() {
        // Test that ISO registry can be created and has distros
        let registry = IsoRegistry::new();
        let distros = registry.get_all_distros();
        assert!(
            !distros.is_empty(),
            "Registry should have at least some distros"
        );

        // Test that common distros are available
        assert!(
            registry.is_supported("ubuntu"),
            "Ubuntu should be supported"
        );
        assert!(
            registry.is_supported("fedora"),
            "Fedora should be supported"
        );
    }

    #[tokio::test]
    async fn test_usb_manager_initialization() {
        // Test that USB manager can be created
        let usb_manager = UsbManager::new();

        // Test device scanning (may return empty list in test environment)
        let result = usb_manager.scan_devices().await;
        assert!(result.is_ok(), "USB scanning should not fail");
    }

    #[test]
    fn test_cli_integration() {
        // Test that CLI commands integrate with helper methods
        use clap::Parser;

        // Test distro name extraction
        let cli = Cli::try_parse_from(["isod", "add", "ubuntu"]).unwrap();
        assert_eq!(cli.get_distro_name(), Some("ubuntu"));

        // Test USB requirement detection
        let cli = Cli::try_parse_from(["isod", "sync"]).unwrap();
        assert!(cli.requires_usb());

        // Test config modification detection
        let cli = Cli::try_parse_from(["isod", "add", "fedora"]).unwrap();
        assert!(cli.modifies_config());
    }

    #[tokio::test]
    async fn test_error_handling() {
        let registry = IsoRegistry::new();

        // Test unsupported distro handling
        assert!(!registry.is_supported("nonexistent-distro"));

        // Test that getting ISO info for unsupported distro fails gracefully
        let result = registry
            .get_iso_info("nonexistent-distro", None, None, None)
            .await;
        assert!(result.is_err(), "Should fail for unsupported distro");
    }

    #[tokio::test]
    async fn test_download_system() {
        use isod::download::{DownloadManager, DownloadOptions};

        // Test that download manager can be created
        let options = DownloadOptions::default();
        let result = DownloadManager::new(options);
        assert!(
            result.is_ok(),
            "Download manager should initialize successfully"
        );
    }
}
