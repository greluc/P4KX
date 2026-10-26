// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use clap::ValueEnum;

/// The values of `p4kx extract --textures`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum TextureOutput {
    /// Merge the header and its parts into one standard DDS file.
    Merge,
    /// Export one mip as an 8-bit RGBA PNG, a high-range texture as OpenEXR.
    Png,
    /// Write the Entries unchanged as well.
    Raw,
}

/// The values of `p4kx extract --audio`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum AudioOutput {
    /// Ogg Vorbis or Ogg Opus rebuilt from Wwise Vorbis or Opus without re-encoding; other codecs
    /// as WAV.
    Ogg,
    /// A 16-bit PCM WAV.
    Wav,
    /// The WEM bytes unchanged.
    Raw,
}
