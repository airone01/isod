use crate::config::ConfigManager;
use crate::registry::IsoRegistry;
use anyhow::Result;
use console::{Term, style};
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
    let term = Term::stdout();

    // Check if distro is supported
    if !iso_registry.is_supported(&distro) {
        term.write_line(&format!(
            "{} Distribution '{}' is not supported.",
            style("❌").red(),
            distro
        ))?;
        term.write_line("")?;
        term.write_line(&format!("{} Available distributions:", style("📋").cyan()))?;
        for d in iso_registry.get_all_distros() {
            if let Some(def) = iso_registry.get_distro(d) {
                term.write_line(&format!(
                    "  {} {} - {}",
                    style("•").dim(),
                    style(d).cyan(),
                    def.display_name
                ))?;
            }
        }
        term.write_line("")?;
        term.write_line(&format!(
            "{} Use 'isod search <term>' to find distributions",
            style("💡").yellow()
        ))?;
        process::exit(1);
    }

    let definition = iso_registry.get_distro(&distro).unwrap();

    // Validate individual variant/arch if specified
    if let Some(ref v) = variant {
        if !definition.supported_variants.contains(v) {
            term.write_line(&format!(
                "{} Variant '{}' not supported for {}.",
                style("❌").red(),
                v,
                distro
            ))?;
            term.write_line(&format!(
                "{} Supported variants: {:?}",
                style("📋").cyan(),
                definition.supported_variants
            ))?;
            process::exit(1);
        }
    }

    if let Some(ref a) = arch {
        if !definition.supported_architectures.contains(a) {
            term.write_line(&format!(
                "{} Architecture '{}' not supported for {}.",
                style("❌").red(),
                a,
                distro
            ))?;
            term.write_line(&format!(
                "{} Supported architectures: {:?}",
                style("📋").cyan(),
                definition.supported_architectures
            ))?;
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
                    term.write_line(&format!(
                        "{} Added variant: {}",
                        style("📦").green(),
                        style(v).cyan()
                    ))?;
                }
            }
        }
    } else if let Some(v) = variant {
        if !distro_config.variants.contains(&v) {
            distro_config.variants.push(v.clone());
            changes_made = true;
            term.write_line(&format!(
                "{} Added variant: {}",
                style("📦").green(),
                style(&v).cyan()
            ))?;
        }
    } else if distro_config.variants.is_empty() {
        if let Some(default_variant) = &definition.default_variant {
            distro_config.variants.push(default_variant.clone());
            changes_made = true;
            term.write_line(&format!(
                "{} Added default variant: {}",
                style("📦").green(),
                style(default_variant).cyan()
            ))?;
        }
    }

    // Handle architectures
    if all_archs {
        for a in &definition.supported_architectures {
            if !distro_config.architectures.contains(a) {
                distro_config.architectures.push(a.clone());
                changes_made = true;
                if verbose {
                    term.write_line(&format!(
                        "{} Added architecture: {}",
                        style("🏗️").green(),
                        style(a).cyan()
                    ))?;
                }
            }
        }
    } else if let Some(a) = arch {
        if !distro_config.architectures.contains(&a) {
            distro_config.architectures.push(a.clone());
            changes_made = true;
            term.write_line(&format!(
                "{} Added architecture: {}",
                style("🏗️").green(),
                style(&a).cyan()
            ))?;
        }
    } else if distro_config.architectures.is_empty() {
        let default_arch = definition
            .supported_architectures
            .first()
            .unwrap_or(&"amd64".to_string())
            .clone();
        distro_config.architectures.push(default_arch.clone());
        changes_made = true;
        term.write_line(&format!(
            "{} Added default architecture: {}",
            style("🏗️").green(),
            style(&default_arch).cyan()
        ))?;
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
        term.write_line(&format!(
            "{} Successfully configured {}",
            style("✅").green(),
            style(&distro).cyan().bold()
        ))?;
    } else {
        term.write_line(&format!(
            "{} {} is already configured with the specified options",
            style("ℹ️").blue(),
            style(&distro).cyan()
        ))?;
    }

    // Show what will be downloaded
    term.write_line("")?;
    term.write_line(&format!(
        "{} Configuration summary for {}:",
        style("📋").cyan(),
        style(&distro).cyan().bold()
    ))?;
    let final_config = config_manager.get_distro_config(&distro).unwrap();
    term.write_line(&format!(
        "   {}: {:?}",
        style("Variants").dim(),
        final_config.variants
    ))?;
    term.write_line(&format!(
        "   {}: {:?}",
        style("Architectures").dim(),
        final_config.architectures
    ))?;

    // Try to show version info
    if verbose {
        term.write_line("")?;
        term.write_line(&format!(
            "{} Checking latest version...",
            style("🔍").cyan()
        ))?;
        match iso_registry.get_latest_version(&distro).await {
            Ok(version_info) => {
                term.write_line(&format!(
                    "   {}: {}",
                    style("Latest version").dim(),
                    style(&version_info.version).green()
                ))?;
                term.write_line(&format!(
                    "   {}: {}",
                    style("Release type").dim(),
                    version_info.release_type
                ))?;
                if let Some(date) = version_info.release_date {
                    term.write_line(&format!("   {}: {}", style("Release date").dim(), date))?;
                }
            }
            Err(e) => {
                if verbose {
                    term.write_line(&format!(
                        "{} Could not fetch version info: {}",
                        style("⚠️").yellow(),
                        e
                    ))?;
                }
            }
        }
    }

    term.write_line("")?;
    term.write_line(&format!(
        "{} Use 'isod update {}' to download the latest version",
        style("💡").yellow(),
        style(&distro).cyan()
    ))?;
    Ok(())
}
