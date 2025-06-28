use crate::config::ConfigManager;
use crate::usb::UsbManager;
use anyhow::Result;
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
    if dry_run {
        println!("🧹 Dry run - showing what would be cleaned");
    } else {
        println!("🧹 Cleaning old versions...");
    }

    println!("📋 Cleanup criteria:");
    println!("   • Keep latest {} versions per distribution", keep);
    println!("   • Minimum age: {} days", min_age);
    if let Some(ref distro) = filter_distro {
        println!("   • Filter: {} only", distro);
    }
    if clean_cache {
        println!("   • Include cache directory");
    }

    let current_device = usb_manager.get_current_device().await;
    if current_device.is_none() {
        eprintln!("❌ No USB device selected.");
        eprintln!("💡 Use 'isod sync' to select a device first");
        process::exit(1);
    }

    println!("🚧 TODO: Implement cleanup logic");
    println!("   Would analyze ISOs and remove old versions based on criteria");

    Ok(())
}
