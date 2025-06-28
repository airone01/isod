use crate::registry::IsoRegistry;
use anyhow::Result;

pub async fn handle_search(
    iso_registry: &IsoRegistry,
    query: String,
    detailed: bool,
    limit: usize,
    verbose: bool,
) -> Result<()> {
    println!("🔍 Searching for distributions matching: '{}'", query);

    let matches = iso_registry.search_distros(&query);

    if matches.is_empty() {
        println!("❌ No distributions found matching '{}'", query);
        println!(
            "💡 Try a broader search term or use 'isod list' to see all available distributions"
        );
        return Ok(());
    }

    let limited_matches: Vec<&str> = matches.clone().into_iter().take(limit).collect();

    println!("📋 Found {} match(es):", limited_matches.len());

    for distro_name in limited_matches {
        if let Some(definition) = iso_registry.get_distro(distro_name) {
            println!("\n📦 {} - {}", distro_name, definition.display_name);

            if detailed || verbose {
                println!("   📝 {}", definition.description);
                println!("   🌐 Homepage: {}", definition.homepage);
                println!(
                    "   🏗️  Architectures: {:?}",
                    definition.supported_architectures
                );
                println!("   📦 Variants: {:?}", definition.supported_variants);

                if verbose {
                    print!("   🔍 Latest version: ");
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                    match iso_registry.get_latest_version(distro_name).await {
                        Ok(version_info) => {
                            println!("{} ({})", version_info.version, version_info.release_type);
                        }
                        Err(_) => {
                            println!("❌ Unable to fetch");
                        }
                    }
                }
            }
        }
    }

    if matches.len() > limit {
        println!(
            "\n📋 Showing {} of {} results. Use --limit to see more.",
            limit,
            matches.len()
        );
    }

    Ok(())
}
