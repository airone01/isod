use crate::registry::IsoRegistry;
use anyhow::Result;
use console::{Term, style};
use indicatif::{ProgressBar, ProgressStyle};
use std::process;
use std::time::Duration;

pub async fn handle_info(
    iso_registry: &IsoRegistry,
    distro: String,
    show_versions: bool,
    show_sources: bool,
    show_details: bool,
    verbose: bool,
) -> Result<()> {
    let term = Term::stdout();
    term.write_line(&format!(
        "{} Information for: {}",
        style("ℹ️").cyan(),
        style(&distro).cyan().bold()
    ))?;

    if !iso_registry.is_supported(&distro) {
        term.write_line(&format!(
            "{} Distribution '{}' is not supported",
            style("❌").red(),
            distro
        ))?;
        term.write_line(&format!(
            "{} Use 'isod search {}' to find similar distributions",
            style("💡").yellow(),
            distro
        ))?;
        process::exit(1);
    }

    let definition = iso_registry.get_distro(&distro).unwrap();

    term.write_line(&format!(
        "\n{} {} - {}",
        style("📦").green(),
        style(&distro).cyan().bold(),
        style(&definition.display_name).green()
    ))?;
    term.write_line(&format!(
        "{} Description: {}",
        style("📝").cyan(),
        definition.description
    ))?;
    term.write_line(&format!(
        "{} Homepage: {}",
        style("🌐").cyan(),
        style(&definition.homepage).cyan()
    ))?;

    if show_details || verbose {
        term.write_line(&format!(
            "\n{} Supported architectures:",
            style("🏗️").cyan()
        ))?;
        for arch in &definition.supported_architectures {
            term.write_line(&format!("   {} {}", style("•").dim(), arch))?;
        }

        term.write_line(&format!("\n{} Supported variants:", style("📦").cyan()))?;
        for variant in &definition.supported_variants {
            term.write_line(&format!("   {} {}", style("•").dim(), variant))?;
        }

        if let Some(default_variant) = &definition.default_variant {
            term.write_line(&format!(
                "   {}: {}",
                style("Default").green(),
                default_variant
            ))?;
        }

        term.write_line(&format!(
            "\n{} Filename pattern: {}",
            style("📁").cyan(),
            style(&definition.filename_pattern).dim()
        ))?;
    }

    if show_versions || verbose {
        term.write_line(&format!(
            "\n{} Checking available versions...",
            style("🔍").cyan()
        ))?;

        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.blue} Fetching version information...")
                .unwrap(),
        );
        spinner.enable_steady_tick(Duration::from_millis(100));

        match iso_registry.get_available_versions(&distro).await {
            Ok(versions) => {
                spinner.finish_and_clear();

                if versions.is_empty() {
                    term.write_line(&format!("{} No versions found", style("❌").red()))?;
                } else {
                    term.write_line(&format!("{} Available versions:", style("📋").cyan()))?;
                    let mut sorted_versions = versions;
                    sorted_versions.sort_by(|a, b| b.cmp(a));

                    for (i, version) in sorted_versions.iter().enumerate() {
                        if !verbose && i >= 5 {
                            term.write_line(&format!(
                                "   {} and {} more (use --verbose to see all)",
                                style("...").dim(),
                                style(sorted_versions.len() - 5).green()
                            ))?;
                            break;
                        }

                        term.write_line(&format!(
                            "   {} {} ({})",
                            style("•").dim(),
                            style(&version.version).green(),
                            style(&version.release_type).blue()
                        ))?;

                        if let Some(date) = &version.release_date {
                            term.write_line(&format!(
                                "     {} Released: {}",
                                style("📅").dim(),
                                date
                            ))?;
                        }
                        if let Some(notes) = &version.notes {
                            term.write_line(&format!("     {} {}", style("📝").dim(), notes))?;
                        }
                    }
                }
            }
            Err(e) => {
                spinner.finish_and_clear();
                term.write_line(&format!(
                    "{} Failed to fetch versions: {}",
                    style("❌").red(),
                    e
                ))?;
            }
        }
    }

    if show_sources || verbose {
        term.write_line(&format!("\n{} Download sources:", style("🌐").cyan()))?;
        for (i, source) in definition.download_sources.iter().enumerate() {
            term.write_line(&format!(
                "   {}. {} ({})",
                style(i + 1).dim(),
                style(&source.source_type).cyan(),
                style(&source.priority).green()
            ))?;

            if let Some(url) = &source.url {
                term.write_line(&format!("      {} {}", style("🔗").dim(), style(url).dim()))?;
            }
            if let Some(desc) = &source.description {
                term.write_line(&format!("      {} {}", style("📝").dim(), desc))?;
            }
            if let Some(region) = &source.region {
                term.write_line(&format!("      {} Region: {}", style("🌍").dim(), region))?;
            }
            if source.verified {
                term.write_line(&format!("      {}", style("✅ Verified").green()))?;
            }
        }
    }

    term.write_line(&format!("\n{} Example commands:", style("💡").yellow()))?;
    term.write_line(&format!(
        "   {}",
        style(&format!("isod add {}", distro)).cyan()
    ))?;
    if let Some(default_variant) = &definition.default_variant {
        term.write_line(&format!(
            "   {}",
            style(&format!(
                "isod add {} --variant {}",
                distro, default_variant
            ))
            .cyan()
        ))?;
    }
    if let Some(arch) = definition.supported_architectures.first() {
        term.write_line(&format!(
            "   {}",
            style(&format!("isod add {} --arch {}", distro, arch)).cyan()
        ))?;
    }
    term.write_line(&format!(
        "   {}",
        style(&format!("isod download {}", distro)).cyan()
    ))?;

    Ok(())
}
