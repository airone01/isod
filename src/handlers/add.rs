use crate::config::ConfigManager;
use crate::registry::IsoRegistry;
use anyhow::Result;
use std::process;

pub async fn handle_add(
    config_manager: &mut ConfigManager,
    iso_registry: &IsoRegistry,
    distro: String,
    variant: Option<String>,
    arch: Option<String>,
    version: Option<String>,
    all_variants: bool,
    all_archs: bool,
    verbose: bool,
) -> Result<()> {
    // Check if distro is supported
    if !iso_registry.is_supported(&distro) {
        eprintln!("❌ Distribution '{}' is not supported.", distro);
        eprintln!("\n📋 Available distributions:");
        for d in iso_registry.get_all_distros() {
            if let Some(def) = iso_registry.get_distro(d) {
                eprintln!("  • {} - {}", d, def.display_name);
            }
        }
        eprintln!("\n💡 Use 'isod search <term>' to find distributions");
        process::exit(1);
    }

    let definition = iso_registry.get_distro(&distro).unwrap();

    // Validate individual variant/arch if specified
    if let Some(ref v) = variant {
        if !definition.supported_variants.contains(v) {
            eprintln!("❌ Variant '{}' not supported for {}.", v, distro);
            eprintln!("📋 Supported variants: {:?}", definition.supported_variants);
            process::exit(1);
        }
    }

    if let Some(ref a) = arch {
        if !definition.supported_architectures.contains(a) {
            eprintln!("❌ Architecture '{}' not supported for {}.", a, distro);
            eprintln!(
                "📋 Supported architectures: {:?}",
                definition.supported_architectures
            );
            process::exit(1);
        }
    }

    // Get or create distro config
    let mut distro_config = config_manager
        .get_distro_config(&distro)
        .cloned()
        .unwrap_or_default();

    let mut changes_made = false;

    // Handle variants
    if all_variants {
        for v in &definition.supported_variants {
            if !distro_config.variants.contains(v) {
                distro_config.variants.push(v.clone());
                changes_made = true;
                if verbose {
                    println!("📦 Added variant: {}", v);
                }
            }
        }
    } else if let Some(v) = variant {
        if !distro_config.variants.contains(&v) {
            distro_config.variants.push(v.clone());
            changes_made = true;
            println!("📦 Added variant: {}", v);
        }
    } else if distro_config.variants.is_empty() {
        if let Some(default_variant) = &definition.default_variant {
            distro_config.variants.push(default_variant.clone());
            changes_made = true;
            println!("📦 Added default variant: {}", default_variant);
        }
    }

    // Handle architectures
    if all_archs {
        for a in &definition.supported_architectures {
            if !distro_config.architectures.contains(a) {
                distro_config.architectures.push(a.clone());
                changes_made = true;
                if verbose {
                    println!("🏗️  Added architecture: {}", a);
                }
            }
        }
    } else if let Some(a) = arch {
        if !distro_config.architectures.contains(&a) {
            distro_config.architectures.push(a.clone());
            changes_made = true;
            println!("🏗️  Added architecture: {}", a);
        }
    } else if distro_config.architectures.is_empty() {
        let default_arch = definition
            .supported_architectures
            .first()
            .unwrap_or(&"amd64".to_string())
            .clone();
        distro_config.architectures.push(default_arch.clone());
        changes_made = true;
        println!("🏗️  Added default architecture: {}", default_arch);
    }

    // Enable the distro
    if !distro_config.enabled {
        distro_config.enabled = true;
        changes_made = true;
    }

    if changes_made {
        // Save updated config
        config_manager.set_distro_config(distro.clone(), distro_config);
        config_manager.save()?;
        println!("✅ Successfully configured {}", distro);
    } else {
        println!(
            "ℹ️  {} is already configured with the specified options",
            distro
        );
    }

    // Show what will be downloaded
    println!("\n📋 Configuration summary for {}:", distro);
    let final_config = config_manager.get_distro_config(&distro).unwrap();
    println!("   Variants: {:?}", final_config.variants);
    println!("   Architectures: {:?}", final_config.architectures);

    // Try to show version info
    if verbose {
        println!("\n🔍 Checking latest version...");
        match iso_registry.get_latest_version(&distro).await {
            Ok(version_info) => {
                println!("   Latest version: {}", version_info.version);
                println!("   Release type: {}", version_info.release_type);
                if let Some(date) = version_info.release_date {
                    println!("   Release date: {}", date);
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("⚠️  Could not fetch version info: {}", e);
                }
            }
        }
    }

    println!(
        "\n💡 Use 'isod update {}' to download the latest version",
        distro
    );
    Ok(())
}
