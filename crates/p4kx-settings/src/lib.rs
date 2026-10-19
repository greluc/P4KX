//! The settings file of P4K-Extractor, a TOML file in the platform config directory.

mod recent;
mod settings;

pub use recent::RecentArchives;
pub use settings::{
    ExtractionAudio, ExtractionTextures, ManualChannels, Settings, SettingsError, WindowGeometry,
    default_settings_file,
};
