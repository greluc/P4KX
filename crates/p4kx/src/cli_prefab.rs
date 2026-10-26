// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::exit::EXIT_CODES_HELP;

/// Arguments of `p4kx prefab`.
#[derive(Debug, Args)]
pub(crate) struct PrefabArgs {
    /// What to do with the prefab libraries.
    #[command(subcommand)]
    pub(crate) action: PrefabAction,
}

/// The actions of `p4kx prefab`.
#[derive(Debug, Subcommand)]
pub(crate) enum PrefabAction {
    /// Lists the prefab libraries below `Data\Prefabs`, or with a LIBRARY its prefabs: name, `Id`,
    /// the number of objects and how many of them instance another prefab. A library is named like
    /// its file below `Data\Prefabs` without `.xml`, ignoring case, with `\` or `/` as separator.
    #[command(after_help = EXIT_CODES_HELP)]
    List(PrefabListArgs),
    /// Prints a prefab with every nested prefab expanded: each object with its type, entity
    /// class, geometry file and transform to the prefab's space, rotation of the parents applied.
    /// A nested prefab is found by its `PrefabGUID`, else by its `PrefabName`. One that is missing,
    /// contains itself, or is nested deeper than 16 levels stays unexpanded and is listed as an
    /// issue. Positions are in metres, Z up, as the libraries store them.
    #[command(after_help = EXIT_CODES_HELP)]
    Dump(PrefabDumpArgs),
}

/// Arguments of `p4kx prefab list`.
#[derive(Debug, Args)]
pub(crate) struct PrefabListArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The library whose prefabs to list; without it, the libraries are listed.
    #[arg(value_name = "LIBRARY")]
    pub(crate) library: Option<String>,
}

/// Arguments of `p4kx prefab dump`.
#[derive(Debug, Args)]
pub(crate) struct PrefabDumpArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The library of the prefab, as `p4kx prefab list` names it.
    #[arg(value_name = "LIBRARY")]
    pub(crate) library: String,
    /// The name of the prefab within its library, ignoring case.
    #[arg(value_name = "PREFAB")]
    pub(crate) prefab: String,
}
