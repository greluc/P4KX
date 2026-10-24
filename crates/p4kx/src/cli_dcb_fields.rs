// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use clap::Args;

use crate::cli::QueryFormat;

/// Arguments of `p4kx dcb fields`.
#[derive(Debug, Args)]
pub(crate) struct DcbFieldsArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The Records to count: a type name, e.g. `EntityClassDefinition`, which selects the Records
    /// of that type and of the types deriving from it, or a Query, defined in docs/query.md, whose
    /// `from`, `at`, `referencing` and `where` clauses select Records and whose `select` clause is
    /// ignored, e.g. `from EntityClassDefinition at "*/spaceships/*"`. Without it every Record is
    /// counted.
    pub(crate) selection: Option<String>,
    /// Lists only the field paths that match, with the rules of `p4kx search`: a glob with `*` or
    /// `?`, otherwise text anywhere in the path; ignores case. Paths are written with `/` between
    /// the field names, e.g. `Components/SCItemWeaponComponentParams/*`.
    #[arg(long, value_name = "GLOB")]
    pub(crate) path: Option<String>,
    /// The format of the result, defined in docs/query.md; defaults to table, or to json with
    /// --json.
    #[arg(long, value_enum)]
    pub(crate) format: Option<QueryFormat>,
}
