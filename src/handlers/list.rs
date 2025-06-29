use anyhow::Result;
use console::{Term, style};
use indicatif::{ProgressBar, ProgressStyle};
use isod::config::ConfigManager;
use isod::registry::IsoRegistry;
use isod::usb::UsbManager;
use std::time::Duration;

pub async fn handle_list(
    config_manager: &ConfigManager,
    iso_registry: &IsoRegistry,
    usb_manager: &UsbManager,
    installed: bool,
    show_versions: bool,
    filter_distro: Option<String>,
    detailed: bool,
) -> Result<()> {
    let term = Term::stdout();

    if installed {
        term.write_line(&format!("{} Installed ISOs:", style("💾").cyan().bold()))?;

        let ventoy_devices = usb_manager.find_ventoy_devices().await?;

        if ventoy_devices.is_empty() {
            term.write_line(&format!("{} No Ventoy devices found.", style("❌").red()))?;
            term.write_line(&format!(
                "{} Make sure your USB device is:",
                style("💡").yellow()
            ))?;
            term.write_line(&format!("   {} Connected and mounted", style("•").dim()))?;
            term.write_line(&format!("   {} Has Ventoy installed", style("•").dim()))?;
            term.write_line(&format!("   {} Is properly formatted", style("•").dim()))?;
            return Ok(());
        }

        for device in ventoy_devices {
            term.write_line(&format!(
                "\n{} Device: {} ({})",
                style("🔌").cyan(),
                style(device.device_path.display()).cyan(),
                device.label.as_deref().unwrap_or("unlabeled")
            ))?;

            if let Some(version) = &device.ventoy_version {
                term.write_line(&format!("   Ventoy version: {}", style(version).green()))?;
            }

            if let Some(mount_point) = &device.mount_point {
                let iso_dir = mount_point.join("iso");
                if iso_dir.exists() {
                    match std::fs::read_dir(&iso_dir) {
                        Ok(entries) => {
                            let mut isos = Vec::new();
                            for entry in entries {
                                if let Ok(entry) = entry {
                                    let path = entry.path();
                                    if path.extension().and_then(|s| s.to_str()) == Some("iso") {
                                        if let Some(name) =
                                            path.file_name().and_then(|s| s.to_str())
                                        {
                                            if let Some(ref filter) = filter_distro {
                                                if name
                                                    .to_lowercase()
                                                    .contains(&filter.to_lowercase())
                                                {
                                                    isos.push((name.to_string(), path.clone()));
                                                }
                                            } else {
                                                isos.push((name.to_string(), path.clone()));
                                            }
                                        }
                                    }
                                }
                            }

                            if isos.is_empty() {
                                if filter_distro.is_some() {
                                    term.write_line(&format!(
                                        "   {} No ISOs found matching filter",
                                        style("📭").dim()
                                    ))?;
                                } else {
                                    term.write_line(&format!(
                                        "   {} No ISO files found",
                                        style("📭").dim()
                                    ))?;
                                }
                            } else {
                                isos.sort_by(|a, b| a.0.cmp(&b.0));
                                for (name, path) in isos {
                                    if detailed {
                                        if let Ok(metadata) = std::fs::metadata(&path) {
                                            let size_gb =
                                                metadata.len() as f64 / (1024.0 * 1024.0 * 1024.0);
                                            term.write_line(&format!(
                                                "   {} {} ({:.1} GB)",
                                                style("📀").green(),
                                                style(&name).cyan(),
                                                size_gb
                                            ))?;
                                        } else {
                                            term.write_line(&format!(
                                                "   {} {}",
                                                style("📀").green(),
                                                style(&name).cyan()
                                            ))?;
                                        }
                                    } else {
                                        term.write_line(&format!(
                                            "   {} {}",
                                            style("📀").green(),
                                            style(&name).cyan()
                                        ))?;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            term.write_line(&format!(
                                "   {} Error reading ISO directory: {}",
                                style("❌").red(),
                                e
                            ))?;
                        }
                    }
                } else {
                    term.write_line(&format!("   {} ISO directory not found", style("📂").dim()))?;
                }
            } else {
                term.write_line(&format!("   {} Device not mounted", style("❌").red()))?;
            }
        }
    } else {
        term.write_line(&format!(
            "{} Available distributions:",
            style("📋").cyan().bold()
        ))?;

        let all_distros = iso_registry.get_all_distros();
        let filtered_distros: Vec<&str> = if let Some(ref filter) = filter_distro {
            all_distros
                .into_iter()
                .filter(|&d| d.contains(&filter.to_lowercase()))
                .collect()
        } else {
            all_distros
        };

        if filtered_distros.is_empty() {
            if filter_distro.is_some() {
                term.write_line(&format!(
                    "{} No distributions found matching filter",
                    style("❌").red()
                ))?;
            } else {
                term.write_line(&format!("{} No distributions available", style("❌").red()))?;
            }
            return Ok(());
        }

        for distro_name in filtered_distros {
            if let Some(definition) = iso_registry.get_distro(distro_name) {
                let configured = config_manager
                    .get_distro_config(distro_name)
                    .map_or(false, |c| c.enabled);

                let status = if configured {
                    style("✅").green()
                } else {
                    style("⬜").dim()
                };
                term.write_line(&format!(
                    "  {} {} - {}",
                    status,
                    style(distro_name).cyan(),
                    definition.display_name
                ))?;

                if detailed {
                    term.write_line(&format!(
                        "     {} {}",
                        style("📝").dim(),
                        definition.description
                    ))?;
                    term.write_line(&format!(
                        "     {} Architectures: {:?}",
                        style("🏗️").dim(),
                        definition.supported_architectures
                    ))?;
                    term.write_line(&format!(
                        "     {} Variants: {:?}",
                        style("📦").dim(),
                        definition.supported_variants
                    ))?;
                    term.write_line(&format!(
                        "     {} Homepage: {}",
                        style("🌐").dim(),
                        definition.homepage
                    ))?;

                    if show_versions {
                        let spinner = ProgressBar::new_spinner();
                        spinner.set_style(
                            ProgressStyle::default_spinner()
                                .template("     {spinner:.blue} Checking versions...")
                                .unwrap(),
                        );
                        spinner.enable_steady_tick(Duration::from_millis(100));

                        match iso_registry.get_latest_version(distro_name).await {
                            Ok(version_info) => {
                                spinner.finish_and_clear();
                                term.write_line(&format!(
                                    "     {} Latest: {} ({})",
                                    style("🔍").cyan(),
                                    style(&version_info.version).green(),
                                    version_info.release_type
                                ))?;
                            }
                            Err(_) => {
                                spinner.finish_and_clear();
                                term.write_line(&format!(
                                    "     {} Unable to fetch",
                                    style("❌").red()
                                ))?;
                            }
                        }
                    }
                    term.write_line("")?;
                }
            }
        }

        term.write_line(&format!(
            "\n{} Configured distributions:",
            style("🛠️").cyan().bold()
        ))?;
        let mut configured_count = 0;
        for (name, config) in &config_manager.config().distros {
            if config.enabled {
                if let Some(ref filter) = filter_distro {
                    if !name.contains(&filter.to_lowercase()) {
                        continue;
                    }
                }

                term.write_line(&format!(
                    "  {} {} - variants: {:?}, architectures: {:?}",
                    style("✅").green(),
                    style(name).cyan(),
                    config.variants,
                    config.architectures
                ))?;
                configured_count += 1;
            }
        }

        if configured_count == 0 {
            if filter_distro.is_some() {
                term.write_line(&format!(
                    "  {} No configured distributions matching filter",
                    style("📭").dim()
                ))?;
            } else {
                term.write_line(&format!("  {} None configured", style("📭").dim()))?;
                term.write_line(&format!(
                    "  {} Use 'isod add <distro>' to add distributions",
                    style("💡").yellow()
                ))?;
            }
        }
    }
    Ok(())
}
