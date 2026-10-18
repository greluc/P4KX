//! The audio of an Archive: the audio triggers Entities name, the ATL definitions that map them
//! to Wwise events and the soundbanks that hold those events.

mod atl;
mod banks;
mod cache;
mod entity;
mod error;
mod hash;
mod index;
mod read;
mod references;

pub use atl::{AtlTrigger, WwiseEvent};
pub use banks::IndexedSoundbank;
pub use cache::ArchiveAudio;
pub use entity::{EntityAudio, EntityTrigger, EventMatch, Unmatched};
pub use error::{AudioFileError, AudioIssue};
pub use hash::wwise_id;
pub use index::AudioIndex;
pub use references::{TriggerReference, trigger_references};

/// The largest ATL definition or soundbank Entry that is read, 256 MiB.
pub const MAX_AUDIO_FILE_LEN: u64 = 256 << 20;

/// The most audio triggers one index holds; the triggers of further ATL files are left out and
/// reported.
pub const MAX_AUDIO_TRIGGERS: usize = 1 << 21;

/// The most Wwise events one ATL trigger posts; further events are left out.
pub const MAX_TRIGGER_EVENTS: usize = 256;

/// The folder of the ATL definition Entries, ignoring ASCII case.
pub const ATL_FOLDER: &str = "Data\\Libs\\GameAudio\\";
