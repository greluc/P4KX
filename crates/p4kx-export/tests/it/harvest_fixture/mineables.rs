// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The mineable entities of the harvest fixture and the radar parameters that name the slot of
//! their scan signature.

use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestValue};

use super::{
    COMPOSITION, CONTACT_TYPE, ENTITY, ENTITY_BARE, ENTITY_SILENT, PARAMS, RADAR_GLOBAL, record,
};

fn signature_component(signatures: &[f32]) -> TestValue {
    let base = TestInstance::new("SSCSignatureSystemBaseSignatureParams").value(
        "signatures",
        TestValue::Array(signatures.iter().copied().map(TestValue::Single).collect()),
    );
    TestValue::Struct(
        TestInstance::new("SSCSignatureSystemParams").value(
            "radarProperties",
            TestValue::Struct(
                TestInstance::new("SSCRadarContactProperites")
                    .value("contactType", TestValue::Reference(CONTACT_TYPE))
                    .value("baseSignatureParams", TestValue::Struct(base)),
            ),
        ),
    )
}

fn mineable_params() -> TestValue {
    TestValue::Struct(
        TestInstance::new("MineableParams")
            .value("composition", TestValue::Reference(COMPOSITION))
            .value("globalParams", TestValue::Reference(PARAMS)),
    )
}

fn mineable_entity(guid: u128, name: &str, components: Vec<TestValue>) -> TestRecord {
    record(
        guid,
        "EntityClassDefinition",
        name,
        &format!(
            "libs/foundry/records/entities/mineable/{}",
            name.to_lowercase()
        ),
    )
    .value("Components", TestValue::Array(components))
}

/// How the radar parameters of the fixture name the scan signature.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Radar {
    /// The third of four signature types is the scan signature.
    Named,
    /// There is no `RadarSystemGlobalParams` Record.
    Missing,
    /// No signature type has the scan signature key.
    Renamed,
    /// Two signature types have the scan signature key.
    Duplicate,
}

/// Returns the radar parameters of `radar`, `None` for a missing Record.
fn radar_global_params(radar: Radar) -> Option<TestRecord> {
    let keys: [&str; 4] = match radar {
        Radar::Named => [
            "@hud_scanning_info_ir_signature",
            "@hud_scanning_info_em_signature",
            "@hud_scanning_info_rs_signature",
            "@hud_scanning_info_db_signature",
        ],
        Radar::Missing => return None,
        Radar::Renamed => [
            "@hud_scanning_info_ir_signature",
            "@hud_scanning_info_em_signature",
            "@hud_scanning_info_scan_signature",
            "@hud_scanning_info_db_signature",
        ],
        Radar::Duplicate => [
            "@hud_scanning_info_rs_signature",
            "@hud_scanning_info_em_signature",
            "@hud_scanning_info_rs_signature",
            "@hud_scanning_info_db_signature",
        ],
    };
    let types = keys.map(|key| {
        TestValue::Struct(
            TestInstance::new("SignatureTypeGlobalParams")
                .value("displayName", TestValue::text(key)),
        )
    });
    Some(
        record(
            RADAR_GLOBAL,
            "RadarSystemGlobalParams",
            "RadarSystemGlobalParams",
            "libs/foundry/records/radarsystem/radarsystemglobalparams",
        )
        .value(
            "signatureSystemParams",
            TestValue::Struct(
                TestInstance::new("SignatureSystemGlobalParams")
                    .value("signatureTypeParams", TestValue::Array(types.to_vec())),
            ),
        ),
    )
}

/// Adds the mineable entities, the radar parameters of `radar` and the contact type to
/// `builder`.
pub(super) fn add(builder: DataForgeBuilder, radar: Radar) -> DataForgeBuilder {
    let builder = match radar_global_params(radar) {
        Some(params) => builder.record(params),
        None => builder,
    };
    builder
        .record(mineable_entity(
            ENTITY,
            "MineableRock_Iron",
            vec![
                mineable_params(),
                signature_component(&[0.0, 0.0, 4270.0, 0.0]),
            ],
        ))
        .record(mineable_entity(
            ENTITY_SILENT,
            "MineableRock_Silent",
            vec![mineable_params(), signature_component(&[0.0; 4])],
        ))
        .record(mineable_entity(
            ENTITY_BARE,
            "MineableRock_Bare",
            Vec::new(),
        ))
        .record(record(
            CONTACT_TYPE,
            "RadarContactTypeEntry",
            "Asteroid",
            "libs/foundry/records/radarsystem/radarcontacttypes/radarcontacttypeentry.asteroid",
        ))
}
