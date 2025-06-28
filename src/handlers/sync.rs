use crate::config::ConfigManager;
use crate::usb::UsbManager;
use anyhow::Result;
use std::process;

pub async fn handle_sync(
    config_manager: &ConfigManager,
    usb_manager: &mut UsbManager,
    mount_point: Option<String>,
    auto_select: bool,
    verify_checksums: bool,
    download_missing: bool,
    verbose: bool,
) -> Result<()> {
    println!("🔄 Syncing with USB device...");

    // Scan for Ventoy devices
    let ventoy_devices = usb_manager.find_ventoy_devices().await?;

    if ventoy_devices.is_empty() {
        eprintln!("❌ No Ventoy devices found.");
        eprintln!("💡 Please ensure:");
        eprintln!("   • USB device is connected");
        eprintln!("   • Device has Ventoy installed");
        eprintln!("   • Device is mounted and accessible");
        process::exit(1);
    }

    // Select device
    let selected_device = if ventoy_devices.len() == 1 || auto_select {
        &ventoy_devices[0]
    } else {
        println!("🔌 Multiple Ventoy devices found:");
        for (i, device) in ventoy_devices.iter().enumerate() {
            println!(
                "  {}. {} ({})",
                i + 1,
                device.device_path.display(),
                device.label.as_deref().unwrap_or("unlabeled")
            );
        }

        print!("❓ Select device [1-{}]: ", ventoy_devices.len());
        std::io::Write::flush(&mut std::io::stdout()).ok();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        let selection: usize = input
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid selection"))?;

        if selection == 0 || selection > ventoy_devices.len() {
            anyhow::bail!("Selection out of range");
        }

        &ventoy_devices[selection - 1]
    };

    println!(
        "✅ Selected device: {} ({})",
        selected_device.device_path.display(),
        selected_device.label.as_deref().unwrap_or("unlabeled")
    );

    if let Some(version) = &selected_device.ventoy_version {
        println!("📦 Ventoy version: {}", version);
    }

    // Validate and select the device
    usb_manager
        .select_device(&selected_device.device_path.to_string_lossy())
        .await?;

    // Create metadata directory
    let metadata_dir = usb_manager.create_isod_metadata_dir().await?;
    if verbose {
        println!("📁 Metadata directory: {:?}", metadata_dir);
    }

    // Show space info
    let available_space = usb_manager.get_available_space().await?;
    let total_space = selected_device.total_space;
    let used_space = total_space - available_space;

    println!("💾 Storage info:");
    println!(
        "   Total: {:.1} GB",
        total_space as f64 / (1024.0 * 1024.0 * 1024.0)
    );
    println!(
        "   Used: {:.1} GB",
        used_space as f64 / (1024.0 * 1024.0 * 1024.0)
    );
    println!(
        "   Available: {:.1} GB",
        available_space as f64 / (1024.0 * 1024.0 * 1024.0)
    );

    if verify_checksums {
        println!("🔍 TODO: Implement checksum verification");
    }

    if download_missing {
        println!("⬇️  TODO: Implement missing ISO download");
    }

    println!("✅ USB sync complete");
    Ok(())
}
