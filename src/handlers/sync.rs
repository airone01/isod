use anyhow::Result;
use console::{Term, style};
use dialoguer::Select;
use isod::config::ConfigManager;
use isod::usb::UsbManager;
use std::process;

pub async fn handle_sync(
    _config_manager: &ConfigManager,
    usb_manager: &mut UsbManager,
    _mount_point: Option<String>,
    auto_select: bool,
    verify_checksums: bool,
    download_missing: bool,
) -> Result<()> {
    let term = Term::stdout();
    term.write_line(&format!(
        "{} Syncing with USB device...",
        style("🔄").cyan()
    ))?;

    // Scan for Ventoy devices
    let ventoy_devices = usb_manager.find_ventoy_devices().await?;

    if ventoy_devices.is_empty() {
        term.write_line(&format!("{} No Ventoy devices found.", style("❌").red()))?;
        term.write_line(&format!("{} Please ensure:", style("💡").yellow()))?;
        term.write_line(&format!("   {} USB device is connected", style("•").dim()))?;
        term.write_line(&format!(
            "   {} Device has Ventoy installed",
            style("•").dim()
        ))?;
        term.write_line(&format!(
            "   {} Device is mounted and accessible",
            style("•").dim()
        ))?;
        process::exit(1);
    }

    // Select device
    let selected_device = if ventoy_devices.len() == 1 || auto_select {
        &ventoy_devices[0]
    } else {
        term.write_line(&format!(
            "{} Multiple Ventoy devices found:",
            style("🔌").cyan()
        ))?;

        let device_options: Vec<String> = ventoy_devices
            .iter()
            .map(|device| {
                format!(
                    "{} ({})",
                    device.device_path.display(),
                    device.label.as_deref().unwrap_or("unlabeled")
                )
            })
            .collect();

        let selection = Select::new()
            .with_prompt("Select device")
            .items(&device_options)
            .default(0)
            .interact()?;

        &ventoy_devices[selection]
    };

    term.write_line(&format!(
        "{} Selected device: {} ({})",
        style("✅").green(),
        style(selected_device.device_path.display()).cyan(),
        selected_device.label.as_deref().unwrap_or("unlabeled")
    ))?;

    if let Some(version) = &selected_device.ventoy_version {
        term.write_line(&format!(
            "{} Ventoy version: {}",
            style("📦").cyan(),
            style(version).green()
        ))?;
    }

    // Validate and select the device
    usb_manager
        .select_device(&selected_device.device_path.to_string_lossy())
        .await?;

    // Create metadata directory
    let metadata_dir = usb_manager.create_isod_metadata_dir().await?;
    term.write_line(&format!(
        "{} Metadata directory: {:?}",
        style("📁").cyan(),
        metadata_dir
    ))?;

    // Show space info
    let available_space = usb_manager.get_available_space().await?;
    let total_space = selected_device.total_space;
    let used_space = total_space - available_space;

    term.write_line(&format!("{} Storage info:", style("💾").cyan()))?;
    term.write_line(&format!(
        "   {}: {:.1} GB",
        style("Total").dim(),
        total_space as f64 / (1024.0 * 1024.0 * 1024.0)
    ))?;
    term.write_line(&format!(
        "   {}: {:.1} GB",
        style("Used").dim(),
        used_space as f64 / (1024.0 * 1024.0 * 1024.0)
    ))?;
    term.write_line(&format!(
        "   {}: {:.1} GB",
        style("Available").dim(),
        available_space as f64 / (1024.0 * 1024.0 * 1024.0)
    ))?;

    if verify_checksums {
        term.write_line(&format!(
            "{} TODO: Implement checksum verification",
            style("🔍").yellow()
        ))?;
    }

    if download_missing {
        term.write_line(&format!(
            "{} TODO: Implement missing ISO download",
            style("⬇️").yellow()
        ))?;
    }

    term.write_line(&format!("{} USB sync complete", style("✅").green()))?;
    Ok(())
}
