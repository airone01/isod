mod cli;
mod config;
mod handlers;
mod registry;
mod usb;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use config::ConfigManager;
use registry::IsoRegistry;
use std::process;
use usb::UsbManager;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    // Validate CLI arguments first
    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    // Initialize logging based on verbosity
    let verbose = args.verbose;
    if args.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    // Initialize systems
    let mut config_manager = ConfigManager::new()?;
    let mut usb_manager = UsbManager::new();
    let iso_registry = IsoRegistry::new();

    // Validate config on startup (unless we're about to fix it)
    let skip_config_validation = handlers::should_skip_config_validation(&args.command);

    if !skip_config_validation {
        if let Err(e) = config_manager.validate() {
            eprintln!("Configuration validation failed: {}", e);
            eprintln!("Run 'isod config validate --fix' to automatically fix common issues");
            eprintln!("Or run 'isod config validate' for detailed validation report");
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
                verbose,
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
                verbose,
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
                long || verbose,
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
                verbose,
            )
            .await?;
        }
        Commands::Config { action } => {
            handlers::handle_config(&mut config_manager, action, verbose).await?;
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
                verbose,
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
                verbose,
            )
            .await?;
        }
        Commands::Search {
            query,
            detailed,
            limit,
        } => {
            handlers::handle_search(&iso_registry, query, detailed, limit, verbose).await?;
        }
        Commands::Info {
            distro,
            versions,
            sources,
            details,
        } => {
            handlers::handle_info(&iso_registry, distro, versions, sources, details, verbose)
                .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init_test_logging() {
        INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .with_test_writer()
                .init();
        });
    }

    #[tokio::test]
    async fn test_config_manager_initialization() {
        init_test_logging();

        // Test that config manager can be created
        let result = ConfigManager::new();
        assert!(
            result.is_ok(),
            "Config manager should initialize successfully"
        );
    }

    #[tokio::test]
    async fn test_iso_registry_initialization() {
        init_test_logging();

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
        init_test_logging();

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
        let cli = Cli::try_parse_from(&["isod", "add", "ubuntu"]).unwrap();
        assert_eq!(cli.get_distro_name(), Some("ubuntu"));

        // Test USB requirement detection
        let cli = Cli::try_parse_from(&["isod", "sync"]).unwrap();
        assert!(cli.requires_usb());

        // Test config modification detection
        let cli = Cli::try_parse_from(&["isod", "add", "fedora"]).unwrap();
        assert!(cli.modifies_config());
    }

    #[tokio::test]
    async fn test_error_handling() {
        init_test_logging();

        let registry = IsoRegistry::new();

        // Test unsupported distro handling
        assert!(!registry.is_supported("nonexistent-distro"));

        // Test that getting ISO info for unsupported distro fails gracefully
        let result = registry
            .get_iso_info("nonexistent-distro", None, None, None)
            .await;
        assert!(result.is_err(), "Should fail for unsupported distro");
    }
}
