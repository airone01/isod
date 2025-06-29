use anyhow::Result;
use console::{Term, style};
use isod::config::ConfigManager;
use isod::usb::UsbManager;
use std::process;

pub async fn handle_clean(
    config_manager: &ConfigManager,
    usb_manager: &UsbManager,
    keep: u32,
    dry_run: bool,
    min_age: u32,
    filter_distro: Option<String>,
    clean_cache: bool,
    verbose: bool,
) -> Result<()> {
    let term = Term::stdout();

    if dry_run {
        term.write_line(&format!(
            "{} Dry run - showing what would be cleaned",
            style("🧹").cyan()
        ))?;
    } else {
        term.write_line(&format!("{} Cleaning old versions...", style("🧹").cyan()))?;
    }

    term.write_line(&format!("{} Cleanup criteria:", style("📋").cyan()))?;
    term.write_line(&format!(
        "   {} Keep latest {} versions per distribution",
        style("•").dim(),
        style(keep).green()
    ))?;
    term.write_line(&format!(
        "   {} Minimum age: {} days",
        style("•").dim(),
        style(min_age).green()
    ))?;
    if let Some(ref distro) = filter_distro {
        term.write_line(&format!(
            "   {} Filter: {} only",
            style("•").dim(),
            style(distro).cyan()
        ))?;
    }
    if clean_cache {
        term.write_line(&format!("   {} Include cache directory", style("•").dim()))?;
    }

    let current_device = usb_manager.get_current_device().await;
    if current_device.is_none() {
        term.write_line(&format!("{} No USB device selected.", style("❌").red()))?;
        term.write_line(&format!(
            "{} Use 'isod sync' to select a device first",
            style("💡").yellow()
        ))?;
        process::exit(1);
    }

    term.write_line(&format!(
        "{} TODO: Implement cleanup logic",
        style("🚧").yellow()
    ))?;
    term.write_line(&format!(
        "   Would analyze ISOs and remove old versions based on criteria"
    ))?;

    Ok(())
}
