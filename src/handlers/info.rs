use crate::registry::IsoRegistry;
use anyhow::Result;
use std::process;

pub async fn handle_info(
    iso_registry: &IsoRegistry,
    distro: String,
    show_versions: bool,
    show_sources: bool,
    show_details: bool,
    verbose: bool,
) -> Result<()> {
    println!("ℹ️  Information for: {}", distro);

    if !iso_registry.is_supported(&distro) {
        eprintln!("❌ Distribution '{}' is not supported", distro);
        eprintln!(
            "💡 Use 'isod search {}' to find similar distributions",
            distro
        );
        process::exit(1);
    }

    let definition = iso_registry.get_distro(&distro).unwrap();

    println!("\n📦 {} - {}", distro, definition.display_name);
    println!("📝 Description: {}", definition.description);
    println!("🌐 Homepage: {}", definition.homepage);

    if show_details || verbose {
        println!("\n🏗️  Supported architectures:");
        for arch in &definition.supported_architectures {
            println!("   • {}", arch);
        }

        println!("\n📦 Supported variants:");
        for variant in &definition.supported_variants {
            println!("   • {}", variant);
        }

        if let Some(default_variant) = &definition.default_variant {
            println!("   Default: {}", default_variant);
        }

        println!("\n📁 Filename pattern: {}", definition.filename_pattern);
    }

    if show_versions || verbose {
        println!("\n🔍 Checking available versions...");
        match iso_registry.get_available_versions(&distro).await {
            Ok(versions) => {
                if versions.is_empty() {
                    println!("❌ No versions found");
                } else {
                    println!("📋 Available versions:");
                    let mut sorted_versions = versions;
                    sorted_versions.sort_by(|a, b| b.cmp(a));

                    for (i, version) in sorted_versions.iter().enumerate() {
                        if !verbose && i >= 5 {
                            println!(
                                "   ... and {} more (use --verbose to see all)",
                                sorted_versions.len() - 5
                            );
                            break;
                        }

                        println!("   • {} ({})", version.version, version.release_type);
                        if let Some(date) = &version.release_date {
                            println!("     📅 Released: {}", date);
                        }
                        if let Some(notes) = &version.notes {
                            println!("     📝 {}", notes);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to fetch versions: {}", e);
            }
        }
    }

    if show_sources || verbose {
        println!("\n🌐 Download sources:");
        for (i, source) in definition.download_sources.iter().enumerate() {
            println!("   {}. {} ({})", i + 1, source.source_type, source.priority);
            if let Some(url) = &source.url {
                println!("      🔗 {}", url);
            }
            if let Some(desc) = &source.description {
                println!("      📝 {}", desc);
            }
            if let Some(region) = &source.region {
                println!("      🌍 Region: {}", region);
            }
            if source.verified {
                println!("      ✅ Verified");
            }
        }
    }

    println!("\n💡 Example commands:");
    println!("   isod add {}", distro);
    if let Some(default_variant) = &definition.default_variant {
        println!("   isod add {} --variant {}", distro, default_variant);
    }
    if let Some(arch) = definition.supported_architectures.first() {
        println!("   isod add {} --arch {}", distro, arch);
    }
    println!("   isod download {}", distro);

    Ok(())
}
