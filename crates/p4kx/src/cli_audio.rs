use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::exit::EXIT_CODES_HELP;

/// Arguments of `p4kx audio`.
#[derive(Debug, Args)]
pub(crate) struct AudioArgs {
    /// What to do with the sound.
    #[command(subcommand)]
    pub(crate) action: AudioAction,
}

/// The actions of `p4kx audio`.
#[derive(Debug, Subcommand)]
pub(crate) enum AudioAction {
    /// Prints the codec, channels, sample rate, duration and loop of a WEM; the loop runs from its
    /// start frame up to, not including, its end frame.
    #[command(after_help = EXIT_CODES_HELP)]
    Info(AudioSource),
    /// Writes a PCM, IMA ADPCM, PTADPCM, Wwise Vorbis or Opus WEM as a 16-bit WAV file with its
    /// loop in a smpl chunk; other codecs fail with an error naming them.
    #[command(after_help = EXIT_CODES_HELP)]
    Wav {
        /// The sound.
        #[command(flatten)]
        source: AudioSource,
        /// The WAV file to write; an existing file is replaced.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
    },
    /// Rebuilds a Wwise Vorbis WEM as an Ogg Vorbis file or an Opus WEM as an Ogg Opus file,
    /// without re-encoding, with its loop in LOOPSTART and LOOPLENGTH comments; other codecs fail
    /// with an error naming them.
    #[command(after_help = EXIT_CODES_HELP)]
    Ogg {
        /// The sound.
        #[command(flatten)]
        source: AudioSource,
        /// The Ogg file to write; an existing file is replaced.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
    },
}

/// Where a WEM sound is.
#[derive(Debug, Args)]
pub(crate) struct AudioSource {
    /// A Data.p4k Archive when ENTRY is given; otherwise the WEM file.
    #[arg(value_name = "ARCHIVE_OR_FILE")]
    pub(crate) path: PathBuf,
    /// Path of the WEM Entry in the Archive, with `\` or `/` as separator, ignoring case.
    pub(crate) entry: Option<String>,
}
