//! The settings file of P4K-Extractor, a TOML file in the platform config directory, and the
//! Archive key the user stores in the same folder.

mod key_choice;
mod key_store;
mod key_text;
mod model_export;
mod queries;
mod recent;
mod settings;

pub use key_choice::{ChosenKey, KeyChoiceError, KeySource, choose_key};
pub use key_store::{KEY_VARIABLE, KeyStore, KeyStoreError};
pub use key_text::KeyText;
pub use model_export::{ModelExportSettings, ModelMaterials};
pub use queries::{NameProblem, SavedQueries, SavedQuery, SavedQueryEdit};
pub use recent::RecentArchives;
pub use settings::{
    ExtractionAudio, ExtractionTextures, MAX_SETTINGS_SIZE, ManualChannels, Settings,
    SettingsError, WindowGeometry, default_settings_file,
};
