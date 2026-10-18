//! The desktop window of P4K-Extractor.
//!
//! The window state lives in [`Model`], separate from the egui code in [`ui`] that draws it, so
//! tests can drive it headless.

mod app;
mod browser;
mod channel_list;
#[cfg(windows)]
pub mod dwm;
mod entry_index;
mod folder_tree;
mod format;
mod language;
mod model;
mod sort;
mod texts;
pub mod theme;
pub mod ui;
pub mod window;

pub use app::App;
pub use browser::{Browser, TreeRow};
pub use channel_list::ChannelList;
pub use entry_index::EntryIndex;
pub use folder_tree::{Folder, FolderId};
pub use language::Language;
pub use model::{Failure, Model};
pub use sort::{Sort, SortColumn, SortDirection};
pub use texts::{Text, Texts};
