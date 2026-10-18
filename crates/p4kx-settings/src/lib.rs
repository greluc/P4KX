//! The settings file of P4K-Extractor, a TOML file in the platform config directory.

mod settings;

pub use settings::{ManualChannels, Settings, SettingsError, default_settings_file};
