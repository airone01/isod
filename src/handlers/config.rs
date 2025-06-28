use crate::cli::ConfigAction;
use crate::config::ConfigManager;
use crate::registry::IsoRegistry;
use crate::usb::UsbManager;
use anyhow::Result;
use std::process;

pub async fn handle_config(
    config_manager: &mut ConfigManager,
    action: ConfigAction,
    verbose: bool,
) -> Result<()> {
    match action {
        ConfigAction::Show { section, format } => {
            let config_content = std::fs::read_to_string(config_manager.config_file())?;

            match section.as_deref() {
                Some("general") => println!("🔧 General configuration:"),
                Some("usb") => println!("💾 USB configuration:"),
                Some("sources") => println!("🌐 Source configuration:"),
                Some("distros") => println!("📦 Distribution configuration:"),
                Some(s) => {
                    eprintln!("❌ Unknown section: {}", s);
                    process::exit(1);
                }
                None => println!("⚙️  Current configuration:"),
            }

            println!("{}", config_content);
        }

        ConfigAction::Edit { editor } => {
            println!(
                "📝 Config file location: {:?}",
                config_manager.config_file()
            );

            let editor_cmd = editor
                .or_else(|| std::env::var("EDITOR").ok())
                .unwrap_or_else(|| {
                    if cfg!(target_os = "windows") {
                        "notepad".to_string()
                    } else {
                        "nano".to_string()
                    }
                });

            println!("🚀 Opening with {}...", editor_cmd);

            let status = std::process::Command::new(&editor_cmd)
                .arg(config_manager.config_file())
                .status()?;

            if status.success() {
                println!("✅ Configuration edited");
                println!("💡 Run 'isod config validate' to check for issues");
            } else {
                eprintln!("❌ Editor exited with error");
            }
        }

        ConfigAction::Validate { fix, warnings } => {
            println!("🔍 Validating configuration...");

            match config_manager.validate() {
                Ok(()) => {
                    println!("✅ Configuration is valid");
                }
                Err(e) => {
                    eprintln!("❌ Configuration validation failed:");
                    eprintln!("   {}", e);

                    if fix {
                        println!("🔧 TODO: Implement automatic fixes");
                    } else {
                        eprintln!("💡 Run with --fix to automatically fix common issues");
                        process::exit(1);
                    }
                }
            }

            if warnings {
                println!("⚠️  TODO: Implement warning checks");
            }
        }

        ConfigAction::Sample { output, force } => {
            let sample_file = if let Some(output_path) = output {
                let path = std::path::PathBuf::from(output_path);
                if path.exists() && !force {
                    eprintln!("❌ File already exists: {:?}", path);
                    eprintln!("💡 Use --force to overwrite");
                    process::exit(1);
                }

                println!("🚧 TODO: Implement custom sample location");
                config_manager.create_sample_config()?
            } else {
                config_manager.create_sample_config()?
            };

            println!("✅ Sample configuration created at: {:?}", sample_file);
        }

        ConfigAction::Set {
            key,
            value,
            value_type,
        } => {
            println!("🔧 Setting {} = {}", key, value);
            if let Some(vt) = value_type {
                println!("🏷️  Value type: {}", vt);
            }
            println!("🚧 TODO: Implement config key setting with proper parsing");
            println!("💡 For now, edit the config file manually with 'isod config edit'");
        }

        ConfigAction::Get { key, format } => {
            println!("🔍 Getting value for key: {}", key);
            println!("📄 Format: {}", format);
            println!("🚧 TODO: Implement config value retrieval");
        }

        ConfigAction::Reset { section, yes } => {
            let target = section.as_deref().unwrap_or("all configuration");

            if !yes {
                print!("❓ Are you sure you want to reset {}? [y/N]: ", target);
                std::io::Write::flush(&mut std::io::stdout()).ok();

                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;

                if !input.trim().to_lowercase().starts_with('y') {
                    println!("❌ Operation cancelled");
                    return Ok(());
                }
            }

            println!("🔄 Resetting {}...", target);
            println!("🚧 TODO: Implement configuration reset");
        }

        ConfigAction::Import { file, merge } => {
            println!("📥 Importing configuration from: {}", file);
            if merge {
                println!("🔀 Merge mode: existing config will be preserved where possible");
            } else {
                println!("🔄 Replace mode: existing config will be overwritten");
            }
            println!("🚧 TODO: Implement configuration import");
        }

        ConfigAction::Export {
            file,
            format,
            documented,
        } => {
            println!("📤 Exporting configuration to: {}", file);
            println!("📄 Format: {}", format);
            if documented {
                println!("📝 Including documentation and comments");
            }
            println!("🚧 TODO: Implement configuration export");
        }
    }
    Ok(())
}
