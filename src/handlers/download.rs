use crate::registry::IsoRegistry;
use anyhow::Result;
use console::{Term, style};
use indicatif::{ProgressBar, ProgressStyle};
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
    verbose: bool,
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

    // Simulate download progress
    term.write_line("")?;
    term.write_line(&format!(
        "{} TODO: Implement actual download logic",
        style("🚧").yellow()
    ))?;

    // Create a demo progress bar to show what it would look like
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .unwrap()
            .progress_chars("#>-")
    );
    pb.set_message(format!("Downloading {}", iso_info.filename));

    // Demo progress
    for _ in 0..100 {
        pb.inc(1);
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    pb.finish_with_message(format!("{} Download complete (demo)", style("✅").green()));

    term.write_line(&format!(
        "   Would download {} to {}",
        style(&iso_info.filename).cyan(),
        style(&download_dir).cyan()
    ))?;

    Ok(())
}
