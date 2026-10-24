// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Own configurations of a Loadout: exclusions and replacements, checked against the types and
//! sizes of the ports.

use std::error::Error;

use p4kx_assembly::{
    Attachment, ChangeError, EntityNode, Incompatibility, LoadoutChanges, LoadoutResolver,
    PortNode, PortSource,
};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{
    DataForgeBuilder, TestEntity, TestLoadout, TestLoadoutEntry, TestPort, entity_schema,
};

type TestResult = Result<(), Box<dyn Error>>;

fn gun(guid: u128, class_name: &str, size: i32) -> TestEntity {
    TestEntity::new(guid, class_name, format!("entities/{class_name}.xml"))
        .attach_type("WeaponGun")
        .sub_type("Gun")
        .size(size)
}

fn item(guid: u128, class_name: &str, attach_type: &str, size: i32) -> TestEntity {
    TestEntity::new(guid, class_name, format!("entities/{class_name}.xml"))
        .attach_type(attach_type)
        .size(size)
}

fn gun_port(name: &str, min: u32, max: u32) -> TestPort {
    TestPort::new(name)
        .sizes(min, max)
        .item_type("WeaponGun", ["Gun"])
}

pub(crate) fn entities() -> Vec<TestEntity> {
    let ship = TestEntity::new(0x1, "TEST_Ship", "entities/test_ship.xml")
        .port(gun_port("weapon_a", 1, 2))
        .port(gun_port("weapon_b", 3, 3).item_type("Turret", Vec::<String>::new()))
        .port(
            TestPort::new("power")
                .sizes(1, 1)
                .item_type("Powerplant", Vec::<String>::new()),
        )
        .loadout(TestLoadout::Manual(vec![
            TestLoadoutEntry::class("weapon_a", "GUN_S1"),
            TestLoadoutEntry::class("weapon_b", "MOUNT").loadout(TestLoadout::Manual(vec![
                TestLoadoutEntry::class("mount_gun", "GUN_S3"),
            ])),
            TestLoadoutEntry::class("power", "PP_S1"),
            TestLoadoutEntry::class("cargo", "CRATE"),
        ]));
    let mount = item(0x2, "MOUNT", "Turret", 3)
        .port(gun_port("mount_gun", 2, 3))
        .loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
            "mount_gun",
            "GUN_S2",
        )]));
    let barrelled = gun(0x3, "GUN_BARRELLED", 2)
        .port(TestPort::new("barrel"))
        .loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
            "barrel", "BARREL",
        )]));
    vec![
        ship,
        mount,
        barrelled,
        gun(0x10, "GUN_S1", 1),
        gun(0x11, "GUN_S2", 2),
        gun(0x12, "GUN_S3", 3),
        gun(0x13, "GUN_POINT_DEFENCE", 1).sub_type("PointDefence"),
        item(0x14, "SHIELD_S1", "Shield", 1),
        item(0x15, "PP_S1", "Powerplant", 1),
        item(0x16, "CRATE", "CargoGrid", 4),
        item(0x17, "BARREL", "Barrel", 1),
        TestEntity::new(0x18, "NO_ITEM", "entities/no_item.xml"),
    ]
}

pub(crate) fn database(entities: &[TestEntity]) -> DataForgeBuilder {
    entities
        .iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        })
}

fn apply(changes: &LoadoutChanges) -> Result<EntityNode, ChangeError> {
    let bytes = database(&entities()).build();
    let forge = DataForge::open(&bytes).map_err(|error| ChangeError::InvalidPort {
        path: error.to_string(),
    })?;
    let mut resolver = LoadoutResolver::new(&forge);
    let ship = resolver
        .entities()
        .find("TEST_Ship")
        .map_err(|error| ChangeError::InvalidPort {
            path: error.to_string(),
        })?;
    resolver.resolve_with(&ship, changes)
}

pub(crate) fn applied(changes: &LoadoutChanges) -> Result<EntityNode, Box<dyn Error>> {
    Ok(apply(changes)?)
}

pub(crate) fn port<'a>(node: &'a EntityNode, path: &str) -> Result<&'a PortNode, Box<dyn Error>> {
    node.walk()
        .into_iter()
        .find(|(found, _)| found == path)
        .map(|(_, port)| port)
        .ok_or_else(|| format!("no port {path}").into())
}

pub(crate) fn held<'a>(
    node: &'a EntityNode,
    path: &str,
) -> Result<Option<&'a str>, Box<dyn Error>> {
    Ok(match port(node, path)?.attachment() {
        Attachment::Entity(child) => Some(child.class_name()),
        _ => None,
    })
}

fn paths(node: &EntityNode) -> Vec<String> {
    node.walk().into_iter().map(|(path, _)| path).collect()
}

#[test]
fn no_changes_give_the_default_loadout() -> TestResult {
    let tree = applied(&LoadoutChanges::new())?;
    assert_eq!(held(&tree, "weapon_a")?, Some("GUN_S1"));
    assert_eq!(held(&tree, "weapon_b/mount_gun")?, Some("GUN_S3"));
    assert_eq!(port(&tree, "weapon_a")?.source(), PortSource::Loadout);
    Ok(())
}

#[test]
fn an_exclusion_removes_the_ports_branch() -> TestResult {
    let tree = applied(&LoadoutChanges::new().exclude("weapon_b"))?;

    let excluded = port(&tree, "weapon_b")?;
    assert_eq!(excluded.source(), PortSource::Excluded);
    assert!(matches!(excluded.attachment(), Attachment::Empty));
    assert!(
        !paths(&tree)
            .iter()
            .any(|path| path.starts_with("weapon_b/")),
        "the mount and its gun are gone: {:?}",
        paths(&tree)
    );
    assert_eq!(held(&tree, "weapon_a")?, Some("GUN_S1"), "the rest stays");
    Ok(())
}

#[test]
fn several_exclusions_apply_and_ignore_case_and_redundancy() -> TestResult {
    let changes = LoadoutChanges::new()
        .exclude("WEAPON_A")
        .exclude("weapon_b")
        .exclude("weapon_b/mount_gun")
        .exclude("cargo");

    let tree = applied(&changes)?;

    for name in ["weapon_a", "weapon_b", "cargo"] {
        assert_eq!(port(&tree, name)?.source(), PortSource::Excluded, "{name}");
    }
    assert_eq!(held(&tree, "power")?, Some("PP_S1"));
    Ok(())
}

#[test]
fn a_nested_port_can_be_excluded_alone() -> TestResult {
    let tree = applied(&LoadoutChanges::new().exclude("weapon_b/mount_gun"))?;

    assert_eq!(held(&tree, "weapon_b")?, Some("MOUNT"), "the mount stays");
    assert_eq!(
        port(&tree, "weapon_b/mount_gun")?.source(),
        PortSource::Excluded
    );
    Ok(())
}

#[test]
fn a_port_the_loadout_lacks_is_unknown() {
    let result = apply(&LoadoutChanges::new().exclude("weapon_c"));
    assert_eq!(
        result.err(),
        Some(ChangeError::UnknownPort {
            path: "weapon_c".to_owned()
        })
    );
    let result = apply(&LoadoutChanges::new().replace("nowhere/at_all", "GUN_S1"));
    assert!(matches!(result, Err(ChangeError::UnknownPort { .. })));
}

#[test]
fn a_compatible_entity_replaces_the_attached_one() -> TestResult {
    let tree = applied(&LoadoutChanges::new().replace("weapon_a", "GUN_S2"))?;

    assert_eq!(held(&tree, "weapon_a")?, Some("GUN_S2"));
    assert_eq!(port(&tree, "weapon_a")?.source(), PortSource::Replaced);
    assert_eq!(held(&tree, "weapon_b")?, Some("MOUNT"), "the rest stays");
    Ok(())
}

#[test]
fn the_entity_is_found_by_class_name_guid_or_record_path() -> TestResult {
    for text in [
        "gun_s2",
        "00000000-0000-0000-0000-000000000011",
        "entities/GUN_S2",
    ] {
        let tree = applied(&LoadoutChanges::new().replace("weapon_a", text))?;
        assert_eq!(held(&tree, "weapon_a")?, Some("GUN_S2"), "{text}");
    }
    Ok(())
}

#[test]
fn the_replacing_entitys_own_loadout_is_resolved() -> TestResult {
    let tree = applied(&LoadoutChanges::new().replace("weapon_a", "GUN_BARRELLED"))?;

    assert_eq!(held(&tree, "weapon_a")?, Some("GUN_BARRELLED"));
    assert_eq!(held(&tree, "weapon_a/barrel")?, Some("BARREL"));
    Ok(())
}

#[test]
fn replacing_an_entity_discards_the_loadout_given_to_the_old_one() -> TestResult {
    let before = applied(&LoadoutChanges::new())?;
    assert_eq!(held(&before, "weapon_b/mount_gun")?, Some("GUN_S3"));
    assert_eq!(
        port(&before, "weapon_b/mount_gun")?.source(),
        PortSource::Override
    );

    let tree = applied(&LoadoutChanges::new().replace("weapon_b", "MOUNT"))?;

    assert_eq!(
        held(&tree, "weapon_b/mount_gun")?,
        Some("GUN_S2"),
        "the mount's own Loadout, not the ship's override"
    );
    assert_eq!(
        port(&tree, "weapon_b/mount_gun")?.source(),
        PortSource::Loadout
    );
    Ok(())
}

#[test]
fn a_port_below_a_replaced_one_can_be_changed_too() -> TestResult {
    let changes = LoadoutChanges::new()
        .replace("weapon_a", "GUN_BARRELLED")
        .exclude("weapon_a/barrel");

    let tree = applied(&changes)?;

    assert_eq!(
        port(&tree, "weapon_a/barrel")?.source(),
        PortSource::Excluded
    );
    Ok(())
}

fn rejected(changes: &LoadoutChanges) -> Result<(String, String, Incompatibility), Box<dyn Error>> {
    match apply(changes) {
        Err(ChangeError::Incompatible {
            path,
            entity,
            reason,
        }) => Ok((path, entity, reason)),
        other => Err(format!("expected a rejection: {other:?}").into()),
    }
}

#[test]
fn an_entity_of_another_type_is_rejected_with_the_reason() -> TestResult {
    let (path, entity, reason) = rejected(&LoadoutChanges::new().replace("weapon_a", "SHIELD_S1"))?;

    assert_eq!((path.as_str(), entity.as_str()), ("weapon_a", "SHIELD_S1"));
    assert_eq!(reason.kind(), "item_type");
    assert_eq!(
        reason.to_string(),
        "its item type is Shield, the port takes WeaponGun"
    );
    Ok(())
}

#[test]
fn an_entity_of_another_sub_type_is_rejected() -> TestResult {
    let (_, _, reason) = rejected(&LoadoutChanges::new().replace("weapon_a", "GUN_POINT_DEFENCE"))?;

    assert_eq!(reason.kind(), "item_sub_type");
    assert_eq!(
        reason.to_string(),
        "its item type WeaponGun has the sub-type PointDefence, the port takes WeaponGun (Gun)"
    );
    Ok(())
}

#[test]
fn an_entity_beyond_the_sizes_is_rejected() -> TestResult {
    let (_, _, large) = rejected(&LoadoutChanges::new().replace("weapon_a", "GUN_S3"))?;
    let (_, _, small) = rejected(&LoadoutChanges::new().replace("weapon_b", "GUN_S1"))?;

    assert_eq!(large, Incompatibility::TooLarge { size: 3, max: 2 });
    assert_eq!(small, Incompatibility::TooSmall { size: 1, min: 3 });
    assert_eq!(
        large.to_string(),
        "its size 3 is above the largest size 2 the port takes"
    );
    Ok(())
}

#[test]
fn an_entity_without_an_item_type_does_not_fit_a_typed_port() -> TestResult {
    let (_, _, reason) = rejected(&LoadoutChanges::new().replace("weapon_a", "NO_ITEM"))?;

    assert_eq!(
        reason.to_string(),
        "its item type is none, the port takes WeaponGun"
    );
    Ok(())
}

#[test]
fn a_port_without_constraints_takes_any_entity() -> TestResult {
    let tree = applied(&LoadoutChanges::new().replace("cargo", "SHIELD_S1"))?;

    assert_eq!(held(&tree, "cargo")?, Some("SHIELD_S1"));
    assert!(port(&tree, "cargo")?.constraints().is_none());
    Ok(())
}

#[test]
fn replacements_that_cannot_be_found_or_are_malformed_are_errors() {
    let missing = apply(&LoadoutChanges::new().replace("weapon_a", "NO_SUCH_GUN"));
    assert!(
        matches!(missing, Err(ChangeError::Entity { .. })),
        "{missing:?}"
    );
    let bad_port = apply(&LoadoutChanges::new().replace("weapon_a//x", "GUN_S1"));
    assert!(matches!(bad_port, Err(ChangeError::InvalidPort { .. })));
    let empty = apply(&LoadoutChanges::new().exclude(" "));
    assert!(matches!(empty, Err(ChangeError::InvalidPort { .. })));
    assert!(matches!(
        LoadoutChanges::new().replace_text("weapon_a"),
        Err(ChangeError::InvalidReplacement { .. })
    ));
    assert!(matches!(
        LoadoutChanges::new().replace_text("=GUN_S1"),
        Err(ChangeError::InvalidReplacement { .. })
    ));
    assert_eq!(
        LoadoutChanges::new().replace_text("weapon_a = GUN_S1").ok(),
        Some(LoadoutChanges::new().replace("weapon_a", "GUN_S1"))
    );
}

#[test]
fn contradicting_changes_are_rejected() {
    let both = apply(
        &LoadoutChanges::new()
            .exclude("weapon_a")
            .replace("weapon_a", "GUN_S1"),
    );
    assert!(
        matches!(both, Err(ChangeError::Conflict { .. })),
        "{both:?}"
    );
    let below = apply(
        &LoadoutChanges::new()
            .exclude("weapon_b")
            .replace("weapon_b/mount_gun", "GUN_S3"),
    );
    assert!(
        matches!(below, Err(ChangeError::Conflict { .. })),
        "{below:?}"
    );
    let twice = apply(
        &LoadoutChanges::new()
            .replace("weapon_a", "GUN_S1")
            .replace("WEAPON_A", "GUN_S2"),
    );
    assert!(
        matches!(twice, Err(ChangeError::Duplicate { .. })),
        "{twice:?}"
    );
}
