// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use anyhow::Result;
use p4kx_assembly::{
    ArchiveFiles, EntityLookupError, EntityNode, LoadoutFileError, LoadoutFiles, LoadoutResolver,
    MAX_XML_LOADOUT_LEN, Piece, Replaced,
};
use serde::Serialize;
use thiserror::Error;

use crate::cli_armour::ArmourGlbArgs;
use crate::query_table::one_line;

const LOCAL_KEY: &str = "local:loadout.xml";

/// A `--loadout` that names no Entity and no XML Loadout file that can be read.
#[derive(Debug, Error)]
#[error("{text:?} is no Entity and no readable XML Loadout: {reason}")]
pub(crate) struct UnreadableLoadout {
    text: String,
    reason: String,
}

/// Where a Loadout given with `--loadout` comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LoadoutSource {
    /// The default Loadout of an Entity Record.
    Entity,
    /// An XML Loadout Entry of the Archive.
    Archive,
    /// An XML Loadout file on disk.
    File,
}

impl LoadoutSource {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Archive => "archive",
            Self::File => "file",
        }
    }
}

/// The XML Loadout files the resolver reads: one file from disk, else the Archive.
pub(crate) struct Files<'a> {
    local: Option<Vec<u8>>,
    archive: ArchiveFiles<'a>,
}

impl<'a> Files<'a> {
    /// Reads `local` when the path `--loadout` names is a file on disk, else only the Archive.
    pub(crate) fn open(archive: ArchiveFiles<'a>, text: Option<&str>) -> Result<Self> {
        let local = match text.map(Path::new).filter(|path| path.is_file()) {
            Some(path) => Some(read_local(path)?),
            None => None,
        };
        Ok(Self { local, archive })
    }

    const fn is_local(&self) -> bool {
        self.local.is_some()
    }
}

impl LoadoutFiles for Files<'_> {
    fn read(&self, path: &str) -> Result<Vec<u8>, LoadoutFileError> {
        match &self.local {
            Some(bytes) if path == LOCAL_KEY => Ok(bytes.clone()),
            _ => self.archive.read(path),
        }
    }
}

fn read_local(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_XML_LOADOUT_LEN.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).is_ok_and(|len| len > MAX_XML_LOADOUT_LEN) {
        return Err(UnreadableLoadout {
            text: path.display().to_string(),
            reason: LoadoutFileError::TooLarge {
                size: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            }
            .to_string(),
        }
        .into());
    }
    Ok(bytes)
}

/// A resolved Loadout and where it came from.
struct Resolved {
    tree: EntityNode,
    source: LoadoutSource,
}

/// The Loadout `--loadout` named, as the report shows it.
#[derive(Debug, Serialize)]
pub(crate) struct LoadoutJson<'a> {
    pub(crate) name: &'a str,
    pub(crate) source: LoadoutSource,
    pub(crate) class_name: Option<String>,
}

/// A piece of the Loadout that a `--piece` took the place of, as the report shows it.
#[derive(Debug, Serialize)]
pub(crate) struct ReplacedJson<'a> {
    pub(crate) name: &'a str,
    pub(crate) by: &'a str,
    pub(crate) item_type: &'a str,
}

impl<'a> From<&'a Replaced> for ReplacedJson<'a> {
    fn from(replaced: &'a Replaced) -> Self {
        Self {
            name: replaced.name(),
            by: replaced.by(),
            item_type: replaced.item_type(),
        }
    }
}

/// The pieces to wear: those of `--loadout`, with the `--piece`s on top.
pub(crate) struct Selection<'a> {
    pub(crate) pieces: Vec<Piece>,
    pub(crate) loadout: Option<LoadoutJson<'a>>,
    pub(crate) replaced: Vec<Replaced>,
}

impl<'a> Selection<'a> {
    /// Finds the Entities of the `--piece`s and resolves the `--loadout`, if there is one.
    pub(crate) fn gather(
        resolver: &mut LoadoutResolver<'_>,
        files: &Files<'_>,
        args: &'a ArmourGlbArgs,
    ) -> Result<Self> {
        let given = args
            .pieces
            .iter()
            .map(|text| piece(resolver, text))
            .collect::<Result<Vec<_>>>()?;
        let Some(text) = &args.loadout else {
            return Ok(Self {
                pieces: given,
                loadout: None,
                replaced: Vec::new(),
            });
        };
        let found = resolve(resolver, files, text)?;
        let (pieces, replaced) = Piece::layer(resolver.worn(&found.tree), given);
        Ok(Self {
            pieces,
            loadout: Some(LoadoutJson {
                name: text,
                source: found.source,
                class_name: (found.source == LoadoutSource::Entity)
                    .then(|| found.tree.class_name().to_owned()),
            }),
            replaced,
        })
    }

    /// Writes the lines of the text report that tell where the pieces come from.
    pub(crate) fn print(&self, out: &mut impl Write) -> io::Result<()> {
        if let Some(loadout) = &self.loadout {
            writeln!(
                out,
                "loadout {} ({})",
                one_line(loadout.name),
                loadout.source.code()
            )?;
        }
        for replaced in &self.replaced {
            writeln!(
                out,
                "replaced {} by {} ({})",
                one_line(replaced.name()),
                one_line(replaced.by()),
                one_line(replaced.item_type())
            )?;
        }
        Ok(())
    }
}

/// Finds the Entity `text` names and reads what it offers to be worn; an Entity that cannot be
/// read stays a piece that the report lists.
fn piece(resolver: &mut LoadoutResolver<'_>, text: &str) -> Result<Piece> {
    let record = resolver.entities().find(text)?;
    Ok(match resolver.wearable(&record) {
        Ok(wearable) => Piece::wearable(wearable),
        Err(error) => Piece::failed(text, error.to_string()),
    })
}

/// Resolves what `--loadout` names: a file on disk, else an Entity by class name, GUID or
/// record path, else an XML Loadout Entry of the Archive.
fn resolve(resolver: &mut LoadoutResolver<'_>, files: &Files<'_>, text: &str) -> Result<Resolved> {
    let unreadable = |error: p4kx_assembly::LoadoutError| UnreadableLoadout {
        text: text.to_owned(),
        reason: error.to_string(),
    };
    if files.is_local() {
        let tree = resolver.resolve_file(LOCAL_KEY).map_err(unreadable)?;
        return Ok(Resolved {
            tree,
            source: LoadoutSource::File,
        });
    }
    match resolver.entities().find(text) {
        Ok(record) => Ok(Resolved {
            tree: resolver.resolve(&record)?,
            source: LoadoutSource::Entity,
        }),
        Err(EntityLookupError::NotFound { .. }) if text.to_ascii_lowercase().ends_with(".xml") => {
            let tree = resolver.resolve_file(text).map_err(unreadable)?;
            Ok(Resolved {
                tree,
                source: LoadoutSource::Archive,
            })
        }
        Err(error) => Err(error.into()),
    }
}
