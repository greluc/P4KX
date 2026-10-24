// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Finds the installed Channels of Star Citizen: in the default Library folder, in the Library
//! folders the RSI Launcher log names, and in manually added folders and Archives.

mod channel;
mod detect;
mod kind;
mod launcher_log;

pub use channel::{Channel, ChannelSource, channel_name};
pub use detect::{SearchPaths, detect, known_library_folders};
pub use kind::ChannelKind;
pub(crate) use launcher_log::library_folders;
