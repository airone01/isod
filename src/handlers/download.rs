use crate::registry::IsoRegistry;
use anyhow::Result;
use std::process;

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
    println!("⬇️  Downloading {} ISO...", distro);

    if !iso_registry.is_supported(&distro) {
        eprintln!("❌ Distribution '{}' is not supported", distro);
        process::exit(1);
    }

    let iso_info = iso_registry
        .get_iso_info(
            &distro,
            version.as_deref(),
            arch.as_deref(),
            variant.as_deref(),
        )
        .await?;

    println!("📦 ISO details:");
    println!("   Distribution: {}", iso_info.distro);
    println!("   Version: {}", iso_info.version);
    println!("   Architecture: {}", iso_info.architecture);
    if let Some(var) = &iso_info.variant {
        println!("   Variant: {}", var);
    }
    println!("   Filename: {}", iso_info.filename);
    println!("   Sources available: {}", iso_info.download_sources.len());

    let download_dir = output_dir.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });
    println!("📁 Download directory: {}", download_dir);

    if prefer_torrent {
        println!("🌊 Torrent downloads preferred");
    }
    println!("🔄 Max concurrent: {}", max_concurrent);
    if verify_checksum {
        println!("✅ Checksum verification enabled");
    }

    println!("🚧 TODO: Implement actual download logic");
    println!(
        "   Would download {} to {}",
        iso_info.filename, download_dir
    );

    Ok(())
}
