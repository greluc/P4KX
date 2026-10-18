//! Localisation of Star Citizen: the `global.ini` file of a language and the resolution of the
//! localisation keys DataForge Records hold, e.g. `@manufacturer_NameACAS`.
//!
//! Each language's texts are the Entry `Data\Localization\<folder>\global.ini`; the folder is
//! `<language>_(<region>)` in lower case, e.g. `german_(germany)`, except plain `english`.
//! [`Language::global_ini_path`] names the Entry of a language's `global.ini` in the Archive;
//! [`GlobalIni::parse`] indexes its bytes without copying the texts, and
//! [`GlobalIni::resolve`] looks a key up in constant time, ignoring ASCII case.

mod error;
mod global_ini;
mod key;
mod language;

pub use error::LocalizationError;
pub use global_ini::{GlobalIni, MAX_GLOBAL_INI_KEYS, MAX_GLOBAL_INI_SIZE};
pub use language::Language;
