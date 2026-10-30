// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{
    CancelToken, DataForge, DataForgeError, Filter, Record, StructValue, TagTree, TreeBuilder,
};
use rayon::prelude::*;
use schemars::JsonSchema;
use serde::Serialize;

use crate::blueprint::as_struct;
use crate::contract::every_field_required;
use crate::guid_text::{CatalogGuid, sort_by_guid};
use crate::mission_layers::{items, key, one};
use crate::record_fields::{boolean, class_name, reference};
use crate::selection::{Kind, is_kind_leftover};
use crate::spawn_model::{TagPaths, TagTerm};
use crate::spawn_read::Reader;
use crate::{CatalogError, CatalogFailure};

/// A mission location template: a `MissionLocationTemplate` Record under
/// `missiondata/pu_locations`, a place a location query can match by its tags.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct MissionLocationTemplate<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `AEGS_Reclaimer_Space_Stanton2c`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The `disabled` flag of the location data, `null` when absent.
    pub(crate) disabled: Option<bool>,
    /// The `isSecurityNetworkHost` flag, `null` when absent.
    pub(crate) is_security_network_host: Option<bool>,
    /// The `generalTags`: what the queries of `contractLocationQueries` match.
    pub(crate) tags: TagPaths,
    /// The `producesTags`.
    pub(crate) produces_tags: TagTerm,
    /// The `consumesTags`.
    pub(crate) consumes_tags: TagTerm,
    /// The `aiSpawnTags`.
    pub(crate) ai_spawn_tags: TagPaths,
    /// The `stringVariants`: the texts the location offers a mission, each under the tag that
    /// names its use.
    pub(crate) names: Vec<LocationName<'a>>,
    /// The `entityClusterMember` Record, `null` when absent.
    pub(crate) entity_cluster_member_guid: Option<CatalogGuid>,
}

/// A string variant of a location template.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct LocationName<'a> {
    /// The tag path of the variant, e.g. `Missions.Text.Name`; `null` when it names none.
    pub(crate) tag: Option<String>,
    /// The localisation key of the text, `null` when it names none. The text itself is not
    /// exported.
    pub(crate) name_key: Option<&'a str>,
}

/// The location templates of a DataForge and what was left out.
pub(crate) struct LocationTemplates<'f> {
    pub(crate) templates: Vec<MissionLocationTemplate<'f>>,
    pub(crate) skipped: usize,
    pub(crate) failures: Vec<CatalogFailure>,
}

enum Piece<'f> {
    Template(Box<MissionLocationTemplate<'f>>),
    Skipped,
    Failed(CatalogFailure),
}

/// Reads the `MissionLocationTemplate` Records `filter` matches, ordered by GUID; a development
/// leftover is left out and counted.
///
/// # Errors
///
/// Returns [`CatalogError::Cancelled`] once `cancel` is cancelled.
pub(crate) fn read<'f>(
    forge: &'f DataForge<'f>,
    tags: &TagTree,
    filter: Option<&Filter>,
    cancel: Option<&CancelToken>,
) -> Result<LocationTemplates<'f>, CatalogError> {
    let rule = Kind::MissionLocationTemplates.records();
    let records: Vec<Record<'f>> = forge
        .records()
        .filter(|record| {
            record
                .record_path()
                .is_ok_and(|path| rule.matches_path(path) && filter.is_none_or(|f| f.matches(path)))
        })
        .collect();
    let pieces: Vec<Piece<'f>> = records
        .par_iter()
        .map_init(TreeBuilder::new, |trees, record| {
            if cancel.is_some_and(CancelToken::is_cancelled) {
                return Err(CatalogError::Cancelled);
            }
            Ok(piece(trees, record, tags)
                .unwrap_or_else(|source| Some(Piece::Failed(CatalogFailure::of(record, &source)))))
        })
        .filter_map(Result::transpose)
        .collect::<Result<_, _>>()?;
    let mut read = LocationTemplates {
        templates: Vec::new(),
        skipped: 0,
        failures: Vec::new(),
    };
    for piece in pieces {
        match piece {
            Piece::Template(template) => read.templates.push(*template),
            Piece::Skipped => read.skipped = read.skipped.saturating_add(1),
            Piece::Failed(failure) => read.failures.push(failure),
        }
    }
    sort_by_guid(&mut read.templates, |template| template.guid);
    Ok(read)
}

fn piece<'f>(
    trees: &mut TreeBuilder<'f>,
    record: &Record<'f>,
    tags: &TagTree,
) -> Result<Option<Piece<'f>>, DataForgeError> {
    let rule = Kind::MissionLocationTemplates.records();
    let path = record.record_path()?;
    if record.type_name()? != rule.type_name {
        return Ok(None);
    }
    if is_kind_leftover(Kind::MissionLocationTemplates, path, record.name()?) {
        return Ok(Some(Piece::Skipped));
    }
    let tree = trees.build(record)?;
    let reader = Reader::new(tags, tree.root());
    let data = one(Some(tree.root()), "locationData");
    Ok(Some(Piece::Template(Box::new(MissionLocationTemplate {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(&tree),
        record_path: path,
        disabled: boolean(data, "disabled"),
        is_security_network_host: boolean(data, "isSecurityNetworkHost"),
        tags: reader.tag_list(data, "generalTags"),
        produces_tags: term(&reader, data, "producesTags"),
        consumes_tags: term(&reader, data, "consumesTags"),
        ai_spawn_tags: reader.tag_list(data, "aiSpawnTags"),
        names: names(&reader, data),
        entity_cluster_member_guid: reference(data, "entityClusterMember"),
    }))))
}

fn term(reader: &Reader<'_, '_>, data: Option<&StructValue<'_>>, name: &str) -> TagTerm {
    let term = one(data, name);
    TagTerm {
        positive_tags: reader.tag_array(term, "positiveTags"),
        negative_tags: reader.tag_array(term, "negativeTags"),
    }
}

fn names<'f>(reader: &Reader<'_, 'f>, data: Option<&StructValue<'f>>) -> Vec<LocationName<'f>> {
    let variants = one(data, "stringVariants");
    items(variants, "variants")
        .iter()
        .filter_map(as_struct)
        .map(|variant| LocationName {
            tag: reader.tag_path(Some(variant), "tag"),
            name_key: key(Some(variant), "string"),
        })
        .collect()
}
