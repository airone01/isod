use crate::config::ConfigManager;
use crate::registry::IsoRegistry;
use crate::usb::UsbManager;
use anyhow::Result;

pub async fn handle_list(
    config_manager: &ConfigManager,
    iso_registry: &IsoRegistry,
    usb_manager: &UsbManager,
    installed: bool,
    show_versions: bool,
    filter_distro: Option<String>,
    detailed: bool,
) -> Result<()> {
    if installed {
        println!("💾 Installed ISOs:");

        let ventoy_devices = usb_manager.find_ventoy_devices().await?;

        if ventoy_devices.is_empty() {
            println!("❌ No Ventoy devices found.");
            println!("💡 Make sure your USB device is:");
            println!("   • Connected and mounted");
            println!("   • Has Ventoy installed");
            println!("   • Is properly formatted");
            return Ok(());
        }

        for device in ventoy_devices {
            println!(
                "\n🔌 Device: {} ({})",
                device.device_path.display(),
                device.label.as_deref().unwrap_or("unlabeled")
            );

            if let Some(version) = &device.ventoy_version {
                println!("   Ventoy version: {}", version);
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
                                    println!("   📭 No ISOs found matching filter");
                                } else {
                                    println!("   📭 No ISO files found");
                                }
                            } else {
                                isos.sort_by(|a, b| a.0.cmp(&b.0));
                                for (name, path) in isos {
                                    if detailed {
                                        if let Ok(metadata) = std::fs::metadata(&path) {
                                            let size_gb =
                                                metadata.len() as f64 / (1024.0 * 1024.0 * 1024.0);
                                            println!("   📀 {} ({:.1} GB)", name, size_gb);
                                        } else {
                                            println!("   📀 {}", name);
                                        }
                                    } else {
                                        println!("   📀 {}", name);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("   ❌ Error reading ISO directory: {}", e);
                        }
                    }
                } else {
                    println!("   📂 ISO directory not found");
                }
            } else {
                println!("   ❌ Device not mounted");
            }
        }
    } else {
        println!("📋 Available distributions:");

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
                println!("❌ No distributions found matching filter");
            } else {
                println!("❌ No distributions available");
            }
            return Ok(());
        }

        for distro_name in filtered_distros {
            if let Some(definition) = iso_registry.get_distro(distro_name) {
                let configured = config_manager
                    .get_distro_config(distro_name)
                    .map_or(false, |c| c.enabled);

                let status = if configured { "✅" } else { "⬜" };
                println!("  {} {} - {}", status, distro_name, definition.display_name);

                if detailed {
                    println!("     📝 {}", definition.description);
                    println!(
                        "     🏗️  Architectures: {:?}",
                        definition.supported_architectures
                    );
                    println!("     📦 Variants: {:?}", definition.supported_variants);
                    println!("     🌐 Homepage: {}", definition.homepage);

                    if show_versions {
                        print!("     🔍 Checking versions... ");
                        std::io::Write::flush(&mut std::io::stdout()).ok();
                        match iso_registry.get_latest_version(distro_name).await {
                            Ok(version_info) => {
                                println!(
                                    "Latest: {} ({})",
                                    version_info.version, version_info.release_type
                                );
                            }
                            Err(_) => {
                                println!("❌ Unable to fetch");
                            }
                        }
                    }
                    println!();
                }
            }
        }

        println!("\n🛠️  Configured distributions:");
        let mut configured_count = 0;
        for (name, config) in &config_manager.config().distros {
            if config.enabled {
                if let Some(ref filter) = filter_distro {
                    if !name.contains(&filter.to_lowercase()) {
                        continue;
                    }
                }

                println!(
                    "  ✅ {} - variants: {:?}, architectures: {:?}",
                    name, config.variants, config.architectures
                );
                configured_count += 1;
            }
        }

        if configured_count == 0 {
            if filter_distro.is_some() {
                println!("  📭 No configured distributions matching filter");
            } else {
                println!("  📭 None configured");
                println!("  💡 Use 'isod add <distro>' to add distributions");
            }
        }
    }
    Ok(())
}
