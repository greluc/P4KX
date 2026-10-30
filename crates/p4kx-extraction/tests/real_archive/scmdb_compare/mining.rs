// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `MineableElement`, `MineableComposition` and `HarvestableClusterPreset`.

use std::collections::{BTreeSet, HashMap};

use serde_json::{Map, Value};

use super::{Findings, elements, reference_guid};

const ELEMENTS: &str = "mining elements";
const COMPOSITIONS: &str = "mining compositions";
const PRESETS: &str = "mining cluster presets";

/// The SCMDB field of a mineable element and the Record field it is read from.
const ELEMENT_FIELDS: [(&str, &str); 7] = [
    ("instability", "elementInstability"),
    ("resistance", "elementResistance"),
    ("optimalWindowMidpoint", "elementOptimalWindowMidpoint"),
    (
        "optimalWindowRandomness",
        "elementOptimalWindowMidpointRandomness",
    ),
    ("optimalWindowThinness", "elementOptimalWindowThinness"),
    ("explosionMultiplier", "elementExplosionMultiplier"),
    ("clusterFactor", "elementClusterFactor"),
];

/// The SCMDB field of a composition part and the Record field it is read from.
const PART_FIELDS: [(&str, &str); 5] = [
    ("probability", "probability"),
    ("minPercent", "minPercentage"),
    ("maxPercent", "maxPercentage"),
    ("curveExponent", "curveExponent"),
    ("qualityScale", "qualityScale"),
];

/// The fields of a cluster parameter, named alike on both sides.
const CLUSTER_FIELDS: [&str; 5] = [
    "relativeProbability",
    "minSize",
    "maxSize",
    "minProximity",
    "maxProximity",
];

/// The maps of the dump, keyed by Record GUID.
const MAPS: [&str; 3] = ["mineableElements", "compositions", "clusteringPresets"];

pub(super) fn record_guids<'d>(dump: &'d Value, guids: &mut BTreeSet<&'d str>) {
    for map in MAPS {
        guids.extend(entries(&dump[map]).map(|(guid, _)| guid.as_str()));
    }
}

pub(super) fn compare(dump: &Value, records: &HashMap<String, Value>, findings: &mut Findings) {
    let [elements_map, compositions, presets] = MAPS.map(|map| &dump[map]);
    for (category, map, compare_one) in [
        (ELEMENTS, elements_map, compare_element as Compare),
        (COMPOSITIONS, compositions, compare_composition),
        (PRESETS, presets, compare_preset),
    ] {
        for (guid, scmdb) in entries(map) {
            let ident = scmdb["name"].as_str().unwrap_or(guid);
            let record = records.get(guid);
            findings.check(category, record.is_some(), || {
                format!("{ident}: {guid} is no Record")
            });
            if let Some(record) = record {
                compare_one(ident, scmdb, record, findings);
            }
        }
    }
}

type Compare = fn(&str, &Value, &Value, &mut Findings);

fn entries(map: &Value) -> impl Iterator<Item = (&String, &Value)> {
    map.as_object().into_iter().flat_map(Map::iter)
}

fn compare_element(ident: &str, scmdb: &Value, record: &Value, findings: &mut Findings) {
    for (field, record_field) in ELEMENT_FIELDS {
        findings.number(
            ELEMENTS,
            ident,
            field,
            scmdb[field].as_f64(),
            record[record_field].as_f64(),
        );
    }
    let resource = reference_guid(&record["resourceType"]);
    findings.check(ELEMENTS, resource == scmdb["resourceType"].as_str(), || {
        format!(
            "{ident}: resourceType SCMDB={} p4kx={resource:?}",
            scmdb["resourceType"]
        )
    });
}

fn compare_composition(ident: &str, scmdb: &Value, record: &Value, findings: &mut Findings) {
    findings.number(
        COMPOSITIONS,
        ident,
        "minimumDistinctElements",
        scmdb["minimumDistinctElements"].as_f64(),
        record["minimumDistinctElements"].as_f64(),
    );
    let parts = elements(&record["compositionArray"]);
    let scmdb_parts = elements(&scmdb["parts"]);
    findings.check(COMPOSITIONS, parts.len() == scmdb_parts.len(), || {
        format!(
            "{ident}: parts SCMDB={} p4kx={}",
            scmdb_parts.len(),
            parts.len()
        )
    });
    if parts.len() != scmdb_parts.len() {
        return;
    }
    for (index, (scmdb, part)) in scmdb_parts.iter().zip(parts).enumerate() {
        let ident = format!("{ident} part {index}");
        let element = reference_guid(&part["mineableElement"]);
        findings.check(
            COMPOSITIONS,
            element == scmdb["elementGuid"].as_str(),
            || {
                format!(
                    "{ident}: element SCMDB={} p4kx={element:?}",
                    scmdb["elementGuid"]
                )
            },
        );
        for (field, record_field) in PART_FIELDS {
            findings.number(
                COMPOSITIONS,
                &ident,
                field,
                scmdb[field].as_f64(),
                part[record_field].as_f64(),
            );
        }
    }
}

fn compare_preset(ident: &str, scmdb: &Value, record: &Value, findings: &mut Findings) {
    findings.number(
        PRESETS,
        ident,
        "probabilityOfClustering",
        scmdb["probabilityOfClustering"].as_f64(),
        record["probabilityOfClustering"].as_f64(),
    );
    let params = elements(&record["clusterParamsArray"]);
    let scmdb_params = elements(&scmdb["params"]);
    findings.check(PRESETS, params.len() == scmdb_params.len(), || {
        format!(
            "{ident}: params SCMDB={} p4kx={}",
            scmdb_params.len(),
            params.len()
        )
    });
    if params.len() != scmdb_params.len() {
        return;
    }
    for (index, (scmdb, param)) in scmdb_params.iter().zip(params).enumerate() {
        let ident = format!("{ident} param {index}");
        for field in CLUSTER_FIELDS {
            findings.number(
                PRESETS,
                &ident,
                field,
                scmdb[field].as_f64(),
                param[field].as_f64(),
            );
        }
    }
}
