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

/// The radar parameters whose third signature type is the scan signature.
fn radar_global_params() -> TestRecord {
    let types = [
        "@hud_scanning_info_ir_signature",
        "@hud_scanning_info_em_signature",
        "@hud_scanning_info_rs_signature",
        "@hud_scanning_info_db_signature",
    ]
    .map(|key| {
        TestValue::Struct(
            TestInstance::new("SignatureTypeGlobalParams")
                .value("displayName", TestValue::text(key)),
        )
    });
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
    )
}

/// Adds the mineable entities, the radar parameters and the contact type to `builder`.
pub(super) fn add(builder: DataForgeBuilder) -> DataForgeBuilder {
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
        .record(radar_global_params())
        .record(record(
            CONTACT_TYPE,
            "RadarContactTypeEntry",
            "Asteroid",
            "libs/foundry/records/radarsystem/radarcontacttypes/radarcontacttypeentry.asteroid",
        ))
}
