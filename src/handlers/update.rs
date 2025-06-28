use crate::config::ConfigManager;
use crate::registry::{IsoRegistry, ReleaseType};
use anyhow::Result;
use console::{Term, style};
use indicatif::{ProgressBar, ProgressStyle};
use std::process;
use std::time::Duration;

pub async fn handle_update(
    config_manager: &ConfigManager,
    iso_registry: &IsoRegistry,
    distro: Option<std::ffi::OsString>,
    force: bool,
    check_only: bool,
    include_beta: bool,
    verbose: bool,
) -> Result<()> {
    let term = Term::stdout();

    match distro {
        Some(d) => {
            let distro_str = d.to_string_lossy();

            if check_only {
                term.write_line(&format!(
                    "{} Checking updates for {}...",
                    style("🔍").cyan(),
                    style(&distro_str).cyan().bold()
                ))?;
            } else {
                term.write_line(&format!(
                    "{} Updating {}{}...",
                    style("⬆️").cyan(),
                    style(&distro_str).cyan().bold(),
                    if force {
                        style(" (forced)").yellow()
                    } else {
                        style("")
                    }
                ))?;
            }

            if !config_manager
                .get_distro_config(&distro_str)
                .map_or(false, |c| c.enabled)
            {
                term.write_line(&format!(
                    "{} {} is not configured.",
                    style("❌").red(),
                    distro_str
                ))?;
                term.write_line(&format!(
                    "{} Add it first with: isod add {}",
                    style("💡").yellow(),
                    style(&distro_str).cyan()
                ))?;
                process::exit(1);
            }

            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.blue} Checking for latest version...")
                    .unwrap(),
            );
            spinner.enable_steady_tick(Duration::from_millis(100));

            match iso_registry.get_latest_version(&distro_str).await {
                Ok(version_info) => {
                    spinner.finish_and_clear();

                    term.write_line(&format!(
                        "{} Latest {} version: {}",
                        style("📦").cyan(),
                        distro_str,
                        style(&version_info.version).green().bold()
                    ))?;
                    term.write_line(&format!(
                        "{} Release type: {}",
                        style("🏷️").cyan(),
                        style(&version_info.release_type).blue()
                    ))?;
                    if let Some(date) = &version_info.release_date {
                        term.write_line(&format!("{} Release date: {}", style("📅").cyan(), date))?;
                    }

                    if check_only {
                        term.write_line(&format!(
                            "{} Use 'isod update {}' to download this version",
                            style("ℹ️").blue(),
                            style(&distro_str).cyan()
                        ))?;
                    } else {
                        term.write_line(&format!(
                            "{} TODO: Implement actual download logic",
                            style("🚧").yellow()
                        ))?;
                        term.write_line(&format!(
                            "   Would download: {}",
                            style(&version_info.version).green()
                        ))?;
                    }
                }
                Err(e) => {
                    spinner.finish_and_clear();
                    term.write_line(&format!(
                        "{} Error checking updates for {}: {}",
                        style("❌").red(),
                        distro_str,
                        e
                    ))?;
                    process::exit(1);
                }
            }
        }
        None => {
            if check_only {
                term.write_line(&format!(
                    "{} Checking updates for all configured distributions...",
                    style("🔍").cyan()
                ))?;
            } else {
                term.write_line(&format!(
                    "{} Updating all configured distributions{}...",
                    style("⬆️").cyan(),
                    if force {
                        style(" (forced)").yellow()
                    } else {
                        style("")
                    }
                ))?;
            }

            let mut update_count = 0;
            let mut error_count = 0;

            for (distro_name, distro_config) in &config_manager.config().distros {
                if !distro_config.enabled {
                    continue;
                }

                term.write_line(&format!(
                    "\n{}",
                    style(&format!("--- {} ---", distro_name)).cyan().bold()
                ))?;

                let spinner = ProgressBar::new_spinner();
                spinner.set_style(
                    ProgressStyle::default_spinner()
                        .template(&format!("{{spinner:.blue}} Checking {}...", distro_name))
                        .unwrap(),
                );
                spinner.enable_steady_tick(Duration::from_millis(100));

                match iso_registry.get_latest_version(distro_name).await {
                    Ok(version_info) => {
                        spinner.finish_and_clear();

                        term.write_line(&format!(
                            "{} Latest version: {}",
                            style("📦").cyan(),
                            style(&version_info.version).green()
                        ))?;
                        term.write_line(&format!(
                            "{} Release type: {}",
                            style("🏷️").cyan(),
                            style(&version_info.release_type).blue()
                        ))?;

                        if include_beta || version_info.release_type == ReleaseType::Stable {
                            update_count += 1;
                            if !check_only {
                                term.write_line(&format!(
                                    "{} TODO: Download {}",
                                    style("🚧").yellow(),
                                    style(&version_info.version).green()
                                ))?;
                            }
                        } else if verbose {
                            term.write_line(&format!(
                                "{} Skipping non-stable release (use --include-beta to include)",
                                style("⏭️").yellow()
                            ))?;
                        }
                    }
                    Err(e) => {
                        spinner.finish_and_clear();
                        term.write_line(&format!(
                            "{} Failed to check {}: {}",
                            style("❌").red(),
                            distro_name,
                            e
                        ))?;
                        error_count += 1;
                    }
                }
            }

            if update_count == 0 && error_count == 0 {
                term.write_line(&format!(
                    "\n{} No distributions configured for updates.",
                    style("📭").dim()
                ))?;
                term.write_line(&format!(
                    "{} Use 'isod add <distro>' to add distributions.",
                    style("💡").yellow()
                ))?;
            } else {
                term.write_line(&format!("\n{} Summary:", style("📊").cyan().bold()))?;
                if check_only {
                    term.write_line(&format!(
                        "   {}: {}",
                        style("Updates available").green(),
                        style(update_count).green().bold()
                    ))?;
                } else {
                    term.write_line(&format!(
                        "   {}: {}",
                        style("Distributions processed").green(),
                        style(update_count).green().bold()
                    ))?;
                }
                if error_count > 0 {
                    term.write_line(&format!(
                        "   {}: {}",
                        style("Errors encountered").red(),
                        style(error_count).red().bold()
                    ))?;
                }
            }
        }
    }
    Ok(())
}
