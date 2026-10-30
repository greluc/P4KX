// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{RecordTree, Value};

use crate::contract::{
    CompositionPart, MineableComposition, MineableElement, MineableEntity, RefiningProcess,
};
use crate::guid_text::CatalogGuid;
use crate::harvest_link::MINEABLE_PARAMS;
use crate::record_fields::{
    class_name, component, elements, integer, locale_key, number, reference, single, structs,
    structure, text,
};
use crate::scan_signature::SignatureSlot;
use crate::texts::Texts;

const SIGNATURE_SYSTEM_PARAMS: &str = "SSCSignatureSystemParams";

/// Returns the mineable entity the Record tree `tree` describes, its scan signature taken from
/// `slot`.
pub(crate) fn mineable_entity<'f>(
    tree: &RecordTree<'f>,
    slot: SignatureSlot,
) -> MineableEntity<'f> {
    let mining = component(tree, MINEABLE_PARAMS);
    let radar = structure(component(tree, SIGNATURE_SYSTEM_PARAMS), "radarProperties");
    let base = structure(radar, "baseSignatureParams");
    let signatures: Vec<f32> = elements(base, "signatures")
        .iter()
        .map(|value| match value {
            Value::Single(signature) if signature.is_finite() => *signature,
            _ => 0.0,
        })
        .collect();
    MineableEntity {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        mineable_composition_guid: reference(mining, "composition"),
        mining_global_params_guid: reference(mining, "globalParams"),
        scan_signature: slot.value(&signatures),
        radar_contact_type_guid: reference(radar, "contactType"),
        signatures,
    }
}

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
