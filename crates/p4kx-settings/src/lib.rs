//! The settings file of P4K-Extractor, a TOML file in the platform config directory.

mod queries;
mod recent;
mod settings;

pub use queries::{NameProblem, SavedQueries, SavedQuery};
pub use recent::RecentArchives;
pub use settings::{
    ExtractionTextures, MAX_SETTINGS_SIZE, ManualChannels, Settings, SettingsError, WindowGeometry,
    default_settings_file,
};
