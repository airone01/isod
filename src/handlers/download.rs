use anyhow::Result;
use console::{Term, style};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use isod::download::{DownloadManager, DownloadOptions, DownloadProgress};
use isod::registry::IsoRegistry;
use std::process;
use std::time::Duration;

pub async fn handle_download(
    iso_registry: &IsoRegistry,
    distro: String,
    output_dir: Option<String>,
    variant: Option<String>,
    arch: Option<String>,
    version: Option<String>,
    prefer_torrent: bool,
    max_concurrent: u8,
    verify_checksum: bool,
) -> Result<()> {
    let term = Term::stdout();
    term.write_line(&format!(
        "{} Downloading {} ISO...",
        style("⬇️").cyan(),
        style(&distro).cyan().bold()
    ))?;

    if !iso_registry.is_supported(&distro) {
        term.write_line(&format!(
            "{} Distribution '{}' is not supported",
            style("❌").red(),
            distro
        ))?;
        process::exit(1);
    }

    // Show a spinner while fetching ISO info
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    spinner.set_message("Fetching ISO information...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let iso_info = iso_registry
        .get_iso_info(
            &distro,
            version.as_deref(),
            arch.as_deref(),
            variant.as_deref(),
        )
        .await?;

    spinner.finish_and_clear();

    term.write_line(&format!("{} ISO details:", style("📦").cyan()))?;
    term.write_line(&format!(
        "   {}: {}",
        style("Distribution").dim(),
        style(&iso_info.distro).cyan()
    ))?;
    term.write_line(&format!(
        "   {}: {}",
        style("Version").dim(),
        style(&iso_info.version).green()
    ))?;
    term.write_line(&format!(
        "   {}: {}",
        style("Architecture").dim(),
        iso_info.architecture
    ))?;
    if let Some(var) = &iso_info.variant {
        term.write_line(&format!("   {}: {}", style("Variant").dim(), var))?;
    }
    term.write_line(&format!(
        "   {}: {}",
        style("Filename").dim(),
        style(&iso_info.filename).cyan()
    ))?;
    term.write_line(&format!(
        "   {}: {}",
        style("Sources available").dim(),
        iso_info.download_sources.len()
    ))?;

    let download_dir = output_dir.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    let options = DownloadOptions {
        max_concurrent: max_concurrent as usize,
        prefer_torrents: prefer_torrent,
        output_directory: download_dir.clone().into(),
        verify_checksums: verify_checksum,
        resume_downloads: true,
    };

    term.write_line(&format!(
        "{} Download directory: {}",
        style("📁").cyan(),
        style(&download_dir).cyan()
    ))?;

    if prefer_torrent {
        term.write_line(&format!(
            "{} Torrent downloads preferred",
            style("🌊").blue()
        ))?;
    }
    term.write_line(&format!(
        "{} Max concurrent: {}",
        style("🔄").cyan(),
        max_concurrent
    ))?;
    if verify_checksum {
        term.write_line(&format!(
            "{} Checksum verification enabled",
            style("✅").green()
        ))?;
    }

    // Fetch checksum if verification is enabled and not already present
    let mut iso_info = iso_info;
    if verify_checksum && iso_info.checksum.is_none() {
        term.write_line(&format!("{} Fetching checksum...", style("🔍").cyan()))?;

        if let Ok(Some(checksum)) = iso_registry.get_checksum(&iso_info).await {
            iso_info.checksum = Some(checksum);
            iso_info.checksum_type = Some("sha256".to_string());
        } else {
            term.write_line(&format!(
                "{} No checksum available for verification",
                style("⚠️").yellow()
            ))?;
        }
    }

    // Create download manager
    let (download_manager, mut progress_receiver) = DownloadManager::new(options.clone())?;

    // Start the download
    term.write_line("")?;
    term.write_line(&format!("{} Starting download...", style("🚀").green()))?;

    let _download_id = download_manager.download_iso(&iso_info, &options).await?;

    // Create progress bar
    let multi_progress = MultiProgress::new();
    let progress_bar = multi_progress.add(ProgressBar::new(100));
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    progress_bar.set_message(format!("Downloading {}", iso_info.filename));

    // Handle progress updates
    let mut download_completed = false;
    while let Some(progress) = progress_receiver.recv().await {
        match progress {
            DownloadProgress::Started { id: _, url, .. } => {
                term.write_line(&format!(
                    "{} Started download from: {}",
                    style("🔗").cyan(),
                    style(&url).dim()
                ))?;
                progress_bar.set_message(format!("Downloading {}", iso_info.filename));
            }
            DownloadProgress::Progress {
                bytes_downloaded,
                total_bytes,
                progress_percent,
                ..
            } => {
                if total_bytes > 0 {
                    progress_bar.set_length(total_bytes);
                    progress_bar.set_position(bytes_downloaded);
                } else {
                    progress_bar.set_position(progress_percent as u64);
                }
            }
            DownloadProgress::VerifyingChecksum { .. } => {
                progress_bar.set_message("Verifying checksum...");
            }
            DownloadProgress::ChecksumVerified { .. } => {
                progress_bar.set_message("Checksum verified ✓");
            }
            DownloadProgress::ChecksumFailed { expected, .. } => {
                progress_bar.finish_with_message(format!(
                    "{} Checksum verification failed! Expected: {}",
                    style("❌").red(),
                    expected
                ));
                term.write_line(&format!(
                    "{} Download completed but checksum verification failed",
                    style("❌").red()
                ))?;
                term.write_line(&format!(
                    "{} The file may be corrupted or tampered with",
                    style("⚠️").yellow()
                ))?;
                process::exit(1);
            }
            DownloadProgress::Completed {
                bytes_downloaded,
                checksum_verified,
                ..
            } => {
                let size_mb = bytes_downloaded as f64 / (1024.0 * 1024.0);
                let checksum_msg = if verify_checksum {
                    if checksum_verified {
                        " (checksum verified ✓)"
                    } else {
                        " (no checksum available)"
                    }
                } else {
                    ""
                };

                progress_bar.finish_with_message(format!(
                    "{} Download complete: {:.1} MB{}",
                    style("✅").green(),
                    size_mb,
                    checksum_msg
                ));

                term.write_line(&format!(
                    "{} Successfully downloaded {} ({:.1} MB){}",
                    style("✅").green(),
                    style(&iso_info.filename).cyan(),
                    size_mb,
                    checksum_msg
                ))?;

                term.write_line(&format!(
                    "{} File saved to: {}",
                    style("📁").cyan(),
                    style(options.output_directory.join(&iso_info.filename).display()).cyan()
                ))?;

                download_completed = true;
                break;
            }
            DownloadProgress::Failed {
                error, attempts, ..
            } => {
                progress_bar.finish_with_message(format!(
                    "{} Download failed after {} attempts",
                    style("❌").red(),
                    attempts
                ));
                term.write_line(&format!("{} Download failed: {}", style("❌").red(), error))?;
                process::exit(1);
            }
            DownloadProgress::Retry {
                attempt,
                max_attempts,
                delay,
                ..
            } => {
                progress_bar.set_message(format!(
                    "Retry {}/{} in {}s...",
                    attempt,
                    max_attempts,
                    delay.as_secs()
                ));
            }
            DownloadProgress::Cancelled { .. } => {
                progress_bar.finish_with_message("Download cancelled");
                term.write_line(&format!("{} Download cancelled", style("❌").red()))?;
                break;
            }
            DownloadProgress::Error { error, .. } => {
                progress_bar.finish_with_message(format!("Error: {}", error));
                term.write_line(&format!("{} Error: {}", style("❌").red(), error))?;
                process::exit(1);
            }
        }
    }

    if !download_completed {
        term.write_line(&format!(
            "{} Download did not complete successfully",
            style("❌").red()
        ))?;
        process::exit(1);
    }

    Ok(())
}
