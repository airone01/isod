use crate::config::ConfigManager;
use crate::registry::{IsoRegistry, ReleaseType};
use anyhow::Result;
use std::process;

pub async fn handle_update(
    config_manager: &ConfigManager,
    iso_registry: &IsoRegistry,
    distro: Option<std::ffi::OsString>,
    force: bool,
    check_only: bool,
    include_beta: bool,
    verbose: bool,
) -> Result<()> {
    match distro {
        Some(d) => {
            let distro_str = d.to_string_lossy();

            if check_only {
                println!("🔍 Checking updates for {}...", distro_str);
            } else {
                println!(
                    "⬆️  Updating {}{}...",
                    distro_str,
                    if force { " (forced)" } else { "" }
                );
            }

            if !config_manager
                .get_distro_config(&distro_str)
                .map_or(false, |c| c.enabled)
            {
                eprintln!("❌ {} is not configured.", distro_str);
                eprintln!("💡 Add it first with: isod add {}", distro_str);
                process::exit(1);
            }

            match iso_registry.get_latest_version(&distro_str).await {
                Ok(version_info) => {
                    println!("📦 Latest {} version: {}", distro_str, version_info.version);
                    println!("🏷️  Release type: {}", version_info.release_type);
                    if let Some(date) = &version_info.release_date {
                        println!("📅 Release date: {}", date);
                    }

                    if check_only {
                        println!(
                            "ℹ️  Use 'isod update {}' to download this version",
                            distro_str
                        );
                    } else {
                        println!("🚧 TODO: Implement actual download logic");
                        println!("   Would download: {}", version_info.version);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error checking updates for {}: {}", distro_str, e);
                    process::exit(1);
                }
            }
        }
        None => {
            if check_only {
                println!("🔍 Checking updates for all configured distributions...");
            } else {
                println!(
                    "⬆️  Updating all configured distributions{}...",
                    if force { " (forced)" } else { "" }
                );
            }

            let mut update_count = 0;
            let mut error_count = 0;

            for (distro_name, distro_config) in &config_manager.config().distros {
                if !distro_config.enabled {
                    continue;
                }

                println!("\n--- {} ---", distro_name);
                match iso_registry.get_latest_version(distro_name).await {
                    Ok(version_info) => {
                        println!("📦 Latest version: {}", version_info.version);
                        println!("🏷️  Release type: {}", version_info.release_type);

                        if include_beta || version_info.release_type == ReleaseType::Stable {
                            update_count += 1;
                            if !check_only {
                                println!("🚧 TODO: Download {}", version_info.version);
                            }
                        } else if verbose {
                            println!(
                                "⏭️  Skipping non-stable release (use --include-beta to include)"
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to check {}: {}", distro_name, e);
                        error_count += 1;
                    }
                }
            }

            if update_count == 0 && error_count == 0 {
                println!("\n📭 No distributions configured for updates.");
                println!("💡 Use 'isod add <distro>' to add distributions.");
            } else {
                println!("\n📊 Summary:");
                if check_only {
                    println!("   Updates available: {}", update_count);
                } else {
                    println!("   Distributions processed: {}", update_count);
                }
                if error_count > 0 {
                    println!("   Errors encountered: {}", error_count);
                }
            }
        }
    }
    Ok(())
}
