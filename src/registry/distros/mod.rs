pub mod arch;
pub mod debian;
pub mod fedora;
pub mod ubuntu;

// Re-export common functionality that distro modules might need
pub use crate::registry::{DistroDefinition, sources::*, version_detection::*};
