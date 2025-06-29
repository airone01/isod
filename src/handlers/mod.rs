pub mod add;
pub mod clean;
pub mod config;
pub mod download;
pub mod info;
pub mod list;
pub mod remove;
pub mod search;
pub mod sync;
pub mod update;

use crate::cli::{Commands, ConfigAction};
use anyhow::Result;
use isod::usb::UsbManager;
use isod::{ConfigManager, IsoRegistry};

// Re-export all handlers
pub use add::handle_add;
pub use clean::handle_clean;
pub use config::handle_config;
pub use download::handle_download;
pub use info::handle_info;
pub use list::handle_list;
pub use remove::handle_remove;
pub use search::handle_search;
pub use sync::handle_sync;
pub use update::handle_update;

/// Check if config validation should be skipped for certain commands
pub fn should_skip_config_validation(command: &Commands) -> bool {
    matches!(
        command,
        Commands::Config {
            action: ConfigAction::Validate { fix: true, .. }
                | ConfigAction::Reset { .. }
                | ConfigAction::Import { .. }
        }
    )
}
