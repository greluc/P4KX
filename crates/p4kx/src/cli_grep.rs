// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use clap::{Args, ValueEnum};
use p4kx_extraction::Language;
use p4kx_search::{DEFAULT_LIMIT, DEFAULT_PER_ITEM, MAX_ENTRY_LEN, MAX_LIMIT, Source};

/// Arguments of `p4kx grep`.
#[derive(Debug, Args)]
pub(crate) struct GrepArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The text to find, as part of a longer text; ignores case and finds `Gladi` in
    /// `AEGS_Gladius`.
    pub(crate) text: String,
    /// Where to search, one or more of records, loc and entries, separated by commas; all three
    /// when not given. `records` searches the string, localisation key and enum values of all
    /// Records, `loc` the texts of each language's global.ini, `entries` the lines of text and
    /// CryXml Entries (CryXml converted to XML first, so line numbers are those of the XML), by
    /// default of the extensions xml, mtl, cdf, adb, entxml, chrparams, animevents, ini, cfg,
    /// json, txt, lua, csv, eco and tbl.
    #[arg(
        long = "in",
        value_enum,
        value_delimiter = ',',
        num_args = 1..,
        value_name = "SOURCE"
    )]
    pub(crate) sources: Vec<GrepSource>,
    /// Narrows the search with a Filter, as `p4kx search` does: with `*` or `?` a glob over the
    /// full text, otherwise text anywhere in it; ignores case, `\` and `/` match alike. It matches
    /// the record path of a Record, the key of a localisation text and the path of an Entry; for
    /// Entries it replaces the list of extensions, so any Entry whose path matches is read.
    #[arg(long, value_name = "FILTER")]
    pub(crate) filter: Option<String>,
    /// The languages of the loc search: a code such as `en`, `de`, `es-419` or `zh-Hans`, or the
    /// folder name of the Archive such as `german_(germany)`, separated by commas; every language
    /// the Archive holds when not given.
    #[arg(
        long,
        value_name = "LANGUAGE",
        value_delimiter = ',',
        num_args = 1..,
        value_parser = clap::value_parser!(Language)
    )]
    pub(crate) lang: Vec<Language>,
    /// The most hits one source reports; the search of the source ends there and says that more
    /// exist.
    #[arg(
        long,
        value_name = "COUNT",
        default_value_t = u32::try_from(DEFAULT_LIMIT).unwrap_or(u32::MAX),
        value_parser = clap::value_parser!(u32).range(1..=i64::try_from(MAX_LIMIT).unwrap_or(i64::MAX))
    )]
    pub(crate) limit: u32,
    /// The most hits one Record or Entry reports.
    #[arg(
        long,
        value_name = "COUNT",
        default_value_t = u32::try_from(DEFAULT_PER_ITEM).unwrap_or(u32::MAX),
        value_parser = clap::value_parser!(u32).range(1..)
    )]
    pub(crate) per_item: u32,
    /// The largest Entry that is read, in bytes; larger Entries are skipped and counted.
    #[arg(long, value_name = "BYTES", default_value_t = MAX_ENTRY_LEN)]
    pub(crate) max_entry_size: u64,
    /// The format of the result, defined in docs/cli-json.md; defaults to table, or to json with
    /// --json.
    #[arg(long, value_enum)]
    pub(crate) format: Option<GrepFormat>,
}

/// The values of `p4kx grep --in`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum GrepSource {
    /// The string, localisation key and enum values of all Records.
    Records,
    /// The texts of the global.ini of each language.
    Loc,
    /// The lines of text and CryXml Entries.
    Entries,
}

impl From<GrepSource> for Source {
    fn from(source: GrepSource) -> Self {
        match source {
            GrepSource::Records => Self::Records,
            GrepSource::Loc => Self::Localization,
            GrepSource::Entries => Self::Entries,
        }
    }
}

/// The values of `p4kx grep --format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum GrepFormat {
    /// One table per source with aligned columns, for reading.
    Table,
    /// One line of JSON.
    Json,
}
