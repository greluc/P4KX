// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::RecordTree;

use crate::contract::{CompositionPart, MineableComposition, MineableElement, RefiningProcess};
use crate::guid_text::CatalogGuid;
use crate::record_fields::{
    class_name, integer, locale_key, number, reference, single, structs, text,
};
use crate::texts::Texts;

/// Returns the mineable composition the Record tree `tree` describes, its name resolved by
/// `texts`.
pub(crate) fn mineable_composition<'f>(
    tree: &RecordTree<'f>,
    texts: &Texts<'f>,
) -> MineableComposition<'f> {
    let root = Some(tree.root());
    MineableComposition {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        deposit_name: texts.localised(locale_key(root, "depositName")),
        minimum_distinct_elements: integer(root, "minimumDistinctElements"),
        parts: structs(root, "compositionArray")
            .map(|part| {
                let part = Some(part);
                CompositionPart {
                    mineable_element_guid: reference(part, "mineableElement"),
                    min_percentage: number(part, "minPercentage"),
                    max_percentage: number(part, "maxPercentage"),
                    probability: number(part, "probability"),
                    curve_exponent: number(part, "curveExponent"),
                    quality_scale: number(part, "qualityScale"),
                }
            })
            .collect(),
    }
}

/// Returns the mineable element the Record tree `tree` describes.
pub(crate) fn mineable_element<'f>(tree: &RecordTree<'f>) -> MineableElement<'f> {
    let root = Some(tree.root());
    MineableElement {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        resource_type_guid: reference(root, "resourceType"),
        instability: single(root, "elementInstability"),
        resistance: single(root, "elementResistance"),
        optimal_window_midpoint: single(root, "elementOptimalWindowMidpoint"),
        optimal_window_midpoint_randomness: single(root, "elementOptimalWindowMidpointRandomness"),
        optimal_window_thinness: single(root, "elementOptimalWindowThinness"),
        explosion_multiplier: single(root, "elementExplosionMultiplier"),
        cluster_factor: single(root, "elementClusterFactor"),
    }
}

/// Returns the refining process the Record tree `tree` describes, its name resolved by `texts`.
pub(crate) fn refining_process<'f>(
    tree: &RecordTree<'f>,
    texts: &Texts<'f>,
) -> RefiningProcess<'f> {
    let root = Some(tree.root());
    RefiningProcess {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        process_name: texts.localised(locale_key(root, "processName")),
        refining_speed: text(root, "refiningSpeed"),
        refining_quality: text(root, "refiningQuality"),
    }
}
