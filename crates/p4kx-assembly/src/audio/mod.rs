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

/// The largest soundbank Entry that is read, 16 MiB.
pub const MAX_SOUNDBANK_LEN: u64 = 16 << 20;

/// The largest ATL definition Entry that is read, 8 MiB.
pub const MAX_ATL_LEN: u64 = 8 << 20;

/// The most nodes a text-XML ATL definition may hold.
pub const MAX_ATL_NODES: u32 = 1 << 20;

/// The most audio triggers one index holds; once it is full, further triggers are left out and
/// the ATL definition that brings the first of them is reported.
pub const MAX_AUDIO_TRIGGERS: usize = 1 << 18;

/// The most Wwise events one ATL trigger posts; further events are left out.
pub const MAX_TRIGGER_EVENTS: usize = 256;

/// The folder of the ATL definition Entries, ignoring ASCII case.
pub const ATL_FOLDER: &str = "Data\\Libs\\GameAudio\\";
