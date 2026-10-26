// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_dataforge::{Guid, Record, StructValue, TagTree, Value, ValueError};

use crate::read::{struct_field, struct_items};
use crate::resolver::LoadoutResolver;
use crate::spec::{LoadoutSpec, read_loadout};

const MAX_OUTFIT_LAYERS: usize = 16;

/// A name tag of an outfit as the Record states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TagSpec {
    /// A Reference to a Tag Record.
    Guid(Guid),
    /// A tag written out as text.
    Name(String),
}

/// One outfit of `SItemPortLoadoutOutfitParams` as the Record states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutfitSpec {
    tagged: bool,
    tags: Vec<TagSpec>,
    files: Vec<String>,
    applied: bool,
}

/// What an Entity's `SItemPortLoadoutOutfitParams` holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutfitInfo {
    base_files: Vec<String>,
    outfits: Vec<OutfitSpec>,
}

struct Candidate {
    spec: OutfitSpec,
    loadouts: Vec<LoadoutSpec>,
}

struct Parsed {
    base: Option<LoadoutSpec>,
    outfits: Vec<Candidate>,
}

/// Returns for each outfit whether it applies, given for each whether it has name tags: every
/// outfit without name tags applies; when all have name tags, the first one does.
fn applied(tagged: &[bool]) -> Vec<bool> {
    let any_untagged = tagged.iter().any(|tagged| !tagged);
    tagged
        .iter()
        .enumerate()
        .map(|(at, tagged)| if any_untagged { !tagged } else { at == 0 })
        .collect()
}

fn parse(outfit: &StructValue<'_>, record: &Arc<str>) -> Parsed {
    let base =
        struct_field(outfit, "BaseLoadout").and_then(|loadout| read_loadout(loadout, record));
    let mut outfits: Vec<Candidate> = struct_items(outfit, "Outfits")
        .map(|candidate| {
            let loadouts: Vec<LoadoutSpec> = struct_items(candidate, "Loadouts")
                .filter_map(|loadout| read_loadout(loadout, record))
                .collect();
            let mut files = Vec::new();
            for loadout in &loadouts {
                loadout.xml_paths(&mut files);
            }
            Candidate {
                spec: OutfitSpec {
                    tagged: has_items(candidate, "OutfitNameTags"),
                    tags: tag_specs(candidate),
                    files,
                    applied: false,
                },
                loadouts,
            }
        })
        .collect();
    let tagged: Vec<bool> = outfits.iter().map(|outfit| outfit.spec.tagged).collect();
    for (outfit, applies) in outfits.iter_mut().zip(applied(&tagged)) {
        outfit.spec.applied = applies;
    }
    Parsed { base, outfits }
}

/// Reads an outfit as layers: the base Loadout, then the Loadouts of the outfits that apply.
pub(crate) fn read_outfit(outfit: &StructValue<'_>, record: &Arc<str>) -> Option<LoadoutSpec> {
    let parsed = parse(outfit, record);
    let layers: Vec<LoadoutSpec> = parsed
        .base
        .into_iter()
        .chain(
            parsed
                .outfits
                .into_iter()
                .filter(|outfit| outfit.spec.applied)
                .flat_map(|outfit| outfit.loadouts),
        )
        .take(MAX_OUTFIT_LAYERS)
        .collect();
    (!layers.is_empty()).then(|| LoadoutSpec::Layered(layers.into()))
}

/// Reads what an outfit holds, for the report.
pub(crate) fn read_info(outfit: &StructValue<'_>, record: &Arc<str>) -> OutfitInfo {
    let parsed = parse(outfit, record);
    let mut base_files = Vec::new();
    if let Some(base) = &parsed.base {
        base.xml_paths(&mut base_files);
    }
    OutfitInfo {
        base_files,
        outfits: parsed
            .outfits
            .into_iter()
            .map(|outfit| outfit.spec)
            .collect(),
    }
}

fn has_items(structure: &StructValue<'_>, name: &str) -> bool {
    matches!(structure.field(name), Some(Value::Array(items)) if !items.is_empty())
}

fn tag_specs(outfit: &StructValue<'_>) -> Vec<TagSpec> {
    match outfit.field("OutfitNameTags") {
        Some(Value::Array(items)) => items.iter().filter_map(tag_spec).collect(),
        _ => Vec::new(),
    }
}

fn tag_spec(value: &Value<'_>) -> Option<TagSpec> {
    match value {
        Value::Reference(link) => Some(TagSpec::Guid(link.guid())),
        Value::Error(ValueError::DanglingReference { guid }) => Some(TagSpec::Guid(*guid)),
        Value::String(text) | Value::Enum(text) | Value::Locale(text) if !text.is_empty() => {
            Some(TagSpec::Name((*text).to_owned()))
        }
        _ => None,
    }
}

impl LoadoutSpec {
    /// Appends the paths of the XML Loadouts this Loadout is made of, in the order they are laid
    /// over each other.
    pub(crate) fn xml_paths(&self, paths: &mut Vec<String>) {
        match self {
            Self::Xml(path) => paths.push(path.to_string()),
            Self::Layered(layers) => {
                for layer in layers.iter() {
                    layer.xml_paths(paths);
                }
            }
            Self::Manual(_) => {}
        }
    }
}

/// One outfit of an Entity's default Loadout: its name tags, the XML Loadouts it adds and whether
/// it is worn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outfit {
    tags: Vec<String>,
    files: Vec<String>,
    applied: bool,
}

impl Outfit {
    /// Returns the tag paths of the outfit's name tags; empty when it has none.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Returns the paths of the XML Loadouts the outfit adds, in order.
    #[must_use]
    pub fn files(&self) -> &[String] {
        &self.files
    }

    /// Returns whether the outfit is worn: every outfit without name tags is; when all outfits
    /// have name tags, the first one is.
    #[must_use]
    pub const fn is_applied(&self) -> bool {
        self.applied
    }
}

/// The outfits of an Entity whose default Loadout is an outfit (`SItemPortLoadoutOutfitParams`):
/// the base Loadout and every outfit, each marked as worn or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutfitReport {
    base_files: Vec<String>,
    outfits: Vec<Outfit>,
}

impl OutfitReport {
    /// Returns the paths of the XML Loadouts of the base Loadout, in order.
    #[must_use]
    pub fn base_files(&self) -> &[String] {
        &self.base_files
    }

    /// Returns the outfits in the order the Record lists them.
    #[must_use]
    pub fn outfits(&self) -> &[Outfit] {
        &self.outfits
    }
}

impl<'f> LoadoutResolver<'f> {
    /// Returns the base Loadout and the outfits of `entity`, or `None` when its default Loadout
    /// is not an outfit or the Record cannot be read.
    pub fn outfits(&mut self, entity: &Record<'f>) -> Option<OutfitReport> {
        let spec = self.spec(entity.guid());
        let info = spec.as_ref().as_ref().ok()?.outfit.as_ref()?;
        let tree = info
            .outfits
            .iter()
            .any(|outfit| {
                outfit
                    .tags
                    .iter()
                    .any(|tag| matches!(tag, TagSpec::Guid(_)))
            })
            .then(|| TagTree::build(self.entities.forge()));
        let name = |tag: &TagSpec| match tag {
            TagSpec::Name(name) => name.clone(),
            TagSpec::Guid(guid) => tree
                .as_ref()
                .and_then(|tree| tree.path_of(*guid))
                .map_or_else(|| guid.to_string(), str::to_owned),
        };
        Some(OutfitReport {
            base_files: info.base_files.clone(),
            outfits: info
                .outfits
                .iter()
                .map(|outfit| Outfit {
                    tags: outfit.tags.iter().map(&name).collect(),
                    files: outfit.files.clone(),
                    applied: outfit.applied,
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::applied;

    #[test]
    fn untagged_outfits_all_apply_and_tagged_ones_do_not() {
        assert_eq!(applied(&[false, true, false]), [true, false, true]);
    }

    #[test]
    fn when_every_outfit_is_tagged_the_first_applies() {
        assert_eq!(applied(&[true, true]), [true, false]);
        assert_eq!(applied(&[true]), [true]);
    }

    #[test]
    fn no_outfit_applies_none() {
        assert_eq!(applied(&[]), [] as [bool; 0]);
    }
}
