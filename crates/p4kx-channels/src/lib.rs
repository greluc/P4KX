//! Finds the installed Channels of Star Citizen: in the default Library folder, in the Library
//! folders the RSI Launcher log names, and in manually added folders and Archives.

mod channel;
mod detect;
mod launcher_log;

pub use channel::{Channel, ChannelSource, channel_name};
pub use detect::{SearchPaths, detect, known_library_folders};
pub(crate) use launcher_log::library_folders;
