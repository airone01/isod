use crate::config::ConfigManager;
use crate::usb::UsbManager;
use anyhow::Result;
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
    println!("🗑️  Removing {} from USB...", distro);

    // Find current USB device
    let current_device = usb_manager.get_current_device().await;
    if current_device.is_none() {
        eprintln!("❌ No USB device selected.");
        eprintln!("💡 Use 'isod sync' to select a device first");
        process::exit(1);
    }

    // Build removal criteria
    let mut criteria = vec![format!("Distribution: {}", distro)];
    if let Some(ref v) = variant {
        criteria.push(format!("Variant: {}", v));
    }
    if let Some(ref ver) = version {
        criteria.push(format!("Version: {}", ver));
    }
    if all {
        criteria.push("Scope: All versions".to_string());
    }

    println!("🎯 Removal criteria:");
    for criterion in &criteria {
        println!("   • {}", criterion);
    }

    // Confirmation prompt
    if !skip_confirmation {
        print!("\n❓ Are you sure you want to remove these ISOs? [y/N]: ");
        std::io::Write::flush(&mut std::io::stdout()).ok();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().to_lowercase().starts_with('y') {
            println!("❌ Operation cancelled");
            return Ok(());
        }
    }

    println!("🚧 TODO: Implement ISO removal from USB");
    println!("   Would remove ISOs matching the specified criteria");

    Ok(())
}
