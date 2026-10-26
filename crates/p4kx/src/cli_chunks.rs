// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::exit::EXIT_CODES_HELP;

/// Arguments of `p4kx chunks`.
#[derive(Debug, Args)]
pub(crate) struct ChunksArgs {
    /// What to do with the chunk file.
    #[command(subcommand)]
    pub(crate) action: ChunksAction,
}

/// The actions of `p4kx chunks`.
#[derive(Debug, Subcommand)]
pub(crate) enum ChunksAction {
    /// Prints the container of a chunk file and every chunk with its index, type, type ID,
    /// version, chunk ID, byte order, offset and size; types not in the table are marked unknown.
    #[command(after_help = EXIT_CODES_HELP)]
    List(ChunkFileSource),
    /// Writes the bytes of one chunk unchanged to a file, or with --text as text.
    #[command(after_help = EXIT_CODES_HELP)]
    Dump {
        /// The chunk file.
        #[command(flatten)]
        source: ChunkFileSource,
        /// The index of the chunk, as `p4kx chunks list` prints it, counted from 0.
        #[arg(long, value_name = "INDEX")]
        chunk: usize,
        /// The file to write; an existing file is replaced.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
        /// Writes the chunk as UTF-8 text: a CryXmlB chunk (type 0x0004 or 0x0014 of a .soc
        /// file) as XML, a JSON chunk or a list of model paths (0x0011, 0x0013) as stored. Other chunk
        /// types have no text form.
        #[arg(long)]
        text: bool,
    },
}

/// Where a chunk file is.
#[derive(Debug, Args)]
pub(crate) struct ChunkFileSource {
    /// A Data.p4k Archive when ENTRY is given; otherwise the chunk file, e.g. a .cgf, .cga, .skin,
    /// .chr, their geometry companion .cgfm, .cgam, .skinm, or a .caf or .dba.
    #[arg(value_name = "ARCHIVE_OR_FILE")]
    pub(crate) path: PathBuf,
    /// Path of the chunk file Entry in the Archive, with `\` or `/` as separator, ignoring case.
    pub(crate) entry: Option<String>,
}
