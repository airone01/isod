use crate::config::ConfigManager;
use crate::usb::UsbManager;
use anyhow::Result;
use console::{Term, style};
use dialoguer::Confirm;
use std::process;

pub async fn handle_remove(
    config_manager: &ConfigManager,
    usb_manager: &UsbManager,
    distro: String,
    variant: Option<String>,
    version: Option<String>,
    all: bool,
    skip_confirmation: bool,
) -> Result<()> {
    let term = Term::stdout();
    term.write_line(&format!(
        "{} Removing {} from USB...",
        style("🗑️").cyan(),
        style(&distro).cyan()
    ))?;

    // Find current USB device
    let current_device = usb_manager.get_current_device().await;
    if current_device.is_none() {
        term.write_line(&format!("{} No USB device selected.", style("❌").red()))?;
        term.write_line(&format!(
            "{} Use 'isod sync' to select a device first",
            style("💡").yellow()
        ))?;
        process::exit(1);
    }

    // Build removal criteria
    let mut criteria = vec![format!("Distribution: {}", style(&distro).cyan())];
    if let Some(ref v) = variant {
        criteria.push(format!("Variant: {}", style(v).cyan()));
    }
    if let Some(ref ver) = version {
        criteria.push(format!("Version: {}", style(ver).cyan()));
    }
    if all {
        criteria.push(format!("Scope: {}", style("All versions").yellow()));
    }

    term.write_line(&format!("{} Removal criteria:", style("🎯").cyan()))?;
    for criterion in &criteria {
        term.write_line(&format!("   {} {}", style("•").dim(), criterion))?;
    }

    // Confirmation prompt
    if !skip_confirmation {
        term.write_line("")?;
        let confirmed = Confirm::new()
            .with_prompt("Are you sure you want to remove these ISOs?")
            .default(false)
            .interact()?;

        if !confirmed {
            term.write_line(&format!("{} Operation cancelled", style("❌").red()))?;
            return Ok(());
        }
    }

    term.write_line(&format!(
        "{} TODO: Implement ISO removal from USB",
        style("🚧").yellow()
    ))?;
    term.write_line(&format!(
        "   Would remove ISOs matching the specified criteria"
    ))?;

    Ok(())
}
