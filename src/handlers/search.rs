use crate::registry::IsoRegistry;
use anyhow::Result;
use console::{Term, style};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub async fn handle_search(
    iso_registry: &IsoRegistry,
    query: String,
    detailed: bool,
    limit: usize,
    verbose: bool,
) -> Result<()> {
    let term = Term::stdout();
    term.write_line(&format!(
        "{} Searching for distributions matching: '{}'",
        style("🔍").cyan(),
        style(&query).cyan().bold()
    ))?;

    let matches = iso_registry.search_distros(&query);

    if matches.is_empty() {
        term.write_line(&format!(
            "{} No distributions found matching '{}'",
            style("❌").red(),
            query
        ))?;
        term.write_line(&format!(
            "{} Try a broader search term or use 'isod list' to see all available distributions",
            style("💡").yellow()
        ))?;
        return Ok(());
    }

    let limited_matches: Vec<&str> = matches.clone().into_iter().take(limit).collect();

    term.write_line(&format!(
        "{} Found {} match(es):",
        style("📋").cyan(),
        style(limited_matches.len()).green().bold()
    ))?;

    for distro_name in limited_matches {
        if let Some(definition) = iso_registry.get_distro(distro_name) {
            term.write_line(&format!(
                "\n{} {} - {}",
                style("📦").green(),
                style(distro_name).cyan().bold(),
                definition.display_name
            ))?;

            if detailed || verbose {
                term.write_line(&format!(
                    "   {} {}",
                    style("📝").dim(),
                    definition.description
                ))?;
                term.write_line(&format!(
                    "   {} Homepage: {}",
                    style("🌐").dim(),
                    style(&definition.homepage).cyan()
                ))?;
                term.write_line(&format!(
                    "   {} Architectures: {:?}",
                    style("🏗️").dim(),
                    definition.supported_architectures
                ))?;
                term.write_line(&format!(
                    "   {} Variants: {:?}",
                    style("📦").dim(),
                    definition.supported_variants
                ))?;

                if verbose {
                    let spinner = ProgressBar::new_spinner();
                    spinner.set_style(
                        ProgressStyle::default_spinner()
                            .template("   {spinner:.blue} Latest version: ")
                            .unwrap(),
                    );
                    spinner.enable_steady_tick(Duration::from_millis(100));

                    match iso_registry.get_latest_version(distro_name).await {
                        Ok(version_info) => {
                            spinner.finish_and_clear();
                            term.write_line(&format!(
                                "   {} Latest version: {} ({})",
                                style("🔍").cyan(),
                                style(&version_info.version).green(),
                                version_info.release_type
                            ))?;
                        }
                        Err(_) => {
                            spinner.finish_and_clear();
                            term.write_line(&format!(
                                "   {} Latest version: Unable to fetch",
                                style("❌").red()
                            ))?;
                        }
                    }
                }
            }
        }
    }

    if matches.len() > limit {
        term.write_line(&format!(
            "\n{} Showing {} of {} results. Use --limit to see more.",
            style("📋").cyan(),
            style(limit).green(),
            style(matches.len()).green()
        ))?;
    }

    Ok(())
}
