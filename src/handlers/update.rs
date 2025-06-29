use anyhow::Result;
use console::{Term, style};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use isod::config::ConfigManager;
use isod::download::{DownloadManager, DownloadOptions, DownloadProgress};
use isod::registry::{IsoRegistry, ReleaseType};
use std::collections::HashMap;
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
    match distro {
        Some(d) => {
            let distro_str = d.to_string_lossy();
            update_single_distro(
                config_manager,
                iso_registry,
                &distro_str,
                force,
                check_only,
                include_beta,
                verbose,
            )
            .await
        }
        None => {
            update_all_distros(
                config_manager,
                iso_registry,
                force,
                check_only,
                include_beta,
                verbose,
            )
            .await
        }
    }
}

async fn update_single_distro(
    config_manager: &ConfigManager,
    iso_registry: &IsoRegistry,
    distro: &str,
    force: bool,
    check_only: bool,
    include_beta: bool,
    verbose: bool,
) -> Result<()> {
    let term = Term::stdout();

    if check_only {
        term.write_line(&format!(
            "{} Checking updates for {}...",
            style("🔍").cyan(),
            style(distro).cyan().bold()
        ))?;
    } else {
        term.write_line(&format!(
            "{} Updating {}{}...",
            style("⬆️").cyan(),
            style(distro).cyan().bold(),
            if force {
                style(" (forced)").yellow()
            } else {
                style("")
            }
        ))?;
    }

    let distro_config = config_manager
        .get_distro_config(distro)
        .filter(|c| c.enabled);

    if distro_config.is_none() {
        term.write_line(&format!(
            "{} {} is not configured.",
            style("❌").red(),
            distro
        ))?;
        term.write_line(&format!(
            "{} Add it first with: isod add {}",
            style("💡").yellow(),
            style(distro).cyan()
        ))?;
        process::exit(1);
    }

    let config = distro_config.unwrap();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} Checking for latest version...")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));

    match iso_registry.get_latest_version(distro).await {
        Ok(version_info) => {
            spinner.finish_and_clear();

            term.write_line(&format!(
                "{} Latest {} version: {}",
                style("📦").cyan(),
                distro,
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
                    style(distro).cyan()
                ))?;
                return Ok(());
            }

            // Check if we should download this version
            let should_download = include_beta
                || matches!(
                    version_info.release_type,
                    ReleaseType::Stable | ReleaseType::LTS
                );

            if !should_download {
                term.write_line(&format!(
                    "{} Skipping non-stable release (use --include-beta to include)",
                    style("⏭️").yellow()
                ))?;
                return Ok(());
            }

            // Download for each configured architecture and variant
            let download_options = DownloadOptions {
                max_concurrent: config_manager.config().general.max_concurrent_downloads as usize,
                prefer_torrents: config_manager.config().general.prefer_torrents,
                output_directory: std::env::current_dir().unwrap_or_default(),
                verify_checksums: true,
                resume_downloads: true,
            };

            let (download_manager, mut progress_receiver) =
                DownloadManager::new(download_options.clone())?;
            let multi_progress = MultiProgress::new();
            let mut active_downloads = HashMap::new();

            for arch in &config.architectures {
                for variant in &config.variants {
                    let iso_info = match iso_registry
                        .get_iso_info(
                            distro,
                            Some(&version_info.version),
                            Some(arch),
                            Some(variant),
                        )
                        .await
                    {
                        Ok(info) => info,
                        Err(e) => {
                            if verbose {
                                term.write_line(&format!(
                                    "{} Skipping {}-{}-{}: {}",
                                    style("⚠️").yellow(),
                                    distro,
                                    arch,
                                    variant,
                                    e
                                ))?;
                            }
                            continue;
                        }
                    };

                    let download_id = download_manager
                        .download_iso(&iso_info, &download_options)
                        .await?;

                    let progress_bar = multi_progress.add(ProgressBar::new(100));
                    progress_bar.set_style(
                        ProgressStyle::default_bar()
                            .template("{spinner:.green} [{elapsed_precise}] [{bar:.cyan/blue}] {bytes}/{total_bytes} {msg}")
                            .unwrap()
                            .progress_chars("#>-")
                    );
                    progress_bar.set_message(format!("{}-{}-{}", distro, arch, variant));

                    active_downloads.insert(download_id, (progress_bar, iso_info));
                }
            }

            if active_downloads.is_empty() {
                term.write_line(&format!("{} No downloads started", style("⚠️").yellow()))?;
                return Ok(());
            }

            term.write_line(&format!(
                "{} Starting {} downloads...",
                style("🚀").green(),
                active_downloads.len()
            ))?;

            // Handle progress updates
            let mut completed_downloads = 0;
            let total_downloads = active_downloads.len();

            while let Some(progress) = progress_receiver.recv().await {
                if let Some((progress_bar, iso_info)) = active_downloads.get(
                    &match &progress {
                        DownloadProgress::Started { id, .. } => id,
                        DownloadProgress::Progress { id, .. } => id,
                        DownloadProgress::VerifyingChecksum { id } => id,
                        DownloadProgress::ChecksumVerified { id } => id,
                        DownloadProgress::ChecksumFailed { id, .. } => id,
                        DownloadProgress::Completed { id, .. } => id,
                        DownloadProgress::Failed { id, .. } => id,
                        DownloadProgress::Retry { id, .. } => id,
                        DownloadProgress::Cancelled { id } => id,
                        DownloadProgress::Error { id, .. } => id,
                    }
                    .clone(),
                ) {
                    match &progress {
                        DownloadProgress::Progress {
                            bytes_downloaded,
                            total_bytes,
                            ..
                        } => {
                            if *total_bytes > 0 {
                                progress_bar.set_length(*total_bytes);
                                progress_bar.set_position(*bytes_downloaded);
                            }
                        }
                        DownloadProgress::VerifyingChecksum { .. } => {
                            progress_bar.set_message("Verifying checksum...");
                        }
                        DownloadProgress::Completed { .. } => {
                            progress_bar.finish_with_message(format!(
                                "{} {}",
                                style("✅").green(),
                                iso_info.filename
                            ));
                            completed_downloads += 1;
                        }
                        DownloadProgress::Failed { error, .. } => {
                            progress_bar.finish_with_message(format!(
                                "{} Failed: {}",
                                style("❌").red(),
                                error
                            ));
                            completed_downloads += 1;
                        }
                        _ => {}
                    }
                }

                if completed_downloads >= total_downloads {
                    break;
                }
            }

            term.write_line(&format!(
                "{} Update complete for {}",
                style("✅").green(),
                style(distro).cyan().bold()
            ))?;
        }
        Err(e) => {
            spinner.finish_and_clear();
            term.write_line(&format!(
                "{} Error checking updates for {}: {}",
                style("❌").red(),
                distro,
                e
            ))?;
            process::exit(1);
        }
    }

    Ok(())
}

async fn update_all_distros(
    config_manager: &ConfigManager,
    iso_registry: &IsoRegistry,
    force: bool,
    check_only: bool,
    include_beta: bool,
    verbose: bool,
) -> Result<()> {
    let term = Term::stdout();

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

        match update_single_distro(
            config_manager,
            iso_registry,
            distro_name,
            force,
            check_only,
            include_beta,
            verbose,
        )
        .await
        {
            Ok(()) => {
                update_count += 1;
            }
            Err(e) => {
                term.write_line(&format!(
                    "{} Failed to update {}: {}",
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

    Ok(())
}
