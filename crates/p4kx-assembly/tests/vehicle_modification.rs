//! The Modification of a vehicle implementation: its Elems and patch files.

mod vehicle_support;

use p4kx_assembly::{LoadoutError, PortVisibility, VehicleImplementation};
use vehicle_support::{
    BASE, DEFINITION, TestResult, files, part_names, resolve, standard_files, visibility,
};

#[test]
fn elems_of_a_modification_clear_invisible_and_restore_skipped_parts() -> TestResult {
    let tree = resolve(DEFINITION, "Open", Some(&standard_files()))?;
    assert!(tree.issues().is_empty(), "issues: {:?}", tree.issues());
    assert_eq!(
        visibility(&tree).first(),
        Some(&("hardpoint_gun".to_owned(), PortVisibility::Visible)),
        "the Elem sets flags on the inner ItemPort by its id"
    );
    let vehicle = tree.vehicle().ok_or("no vehicle")?;
    assert_eq!(vehicle.modification(), Some("Open"));
    assert_eq!(
        part_names(vehicle.item_ports()),
        ["Hardpoint_Gun", "hardpoint_seat", "hardpoint_extra"],
        "skipPart 0 brings the part back"
    );
    let gun = vehicle.item_ports().next().ok_or("no port part")?;
    assert_eq!(gun.damage_max(), Some(300.0), "attribute names ignore case");
    Ok(())
}

#[test]
fn a_patch_file_with_ids_merges_into_the_matching_elements() -> TestResult {
    let tree = resolve(DEFINITION, "Patched", Some(&standard_files()))?;
    assert!(tree.issues().is_empty(), "issues: {:?}", tree.issues());
    assert_eq!(
        visibility(&tree).first(),
        Some(&("hardpoint_gun".to_owned(), PortVisibility::Visible))
    );
    let vehicle = tree.vehicle().ok_or("no vehicle")?;
    assert_eq!(
        vehicle.patch_path(),
        Some("Scripts/Entities/Vehicles/Implementations/Xml/Modifications/TEST_Ship_Patched.xml")
    );
    assert_eq!(
        part_names(vehicle.item_ports()),
        ["Hardpoint_Gun", "hardpoint_seat"],
        "the rest of the tree stays"
    );
    Ok(())
}

#[test]
fn a_parts_only_patch_file_replaces_the_part_tree() -> TestResult {
    let tree = resolve(DEFINITION, "PartsOnly", Some(&standard_files()))?;
    assert!(tree.issues().is_empty(), "issues: {:?}", tree.issues());
    let vehicle = tree.vehicle().ok_or("no vehicle")?;
    assert_eq!(part_names(vehicle.item_ports()), ["hardpoint_new"]);
    assert_eq!(
        vehicle
            .parts()
            .first()
            .and_then(p4kx_assembly::VehiclePart::mass),
        Some(900.0)
    );
    assert_eq!(
        visibility(&tree),
        [
            ("hardpoint_gun".to_owned(), PortVisibility::Unspecified),
            ("hardpoint_seat".to_owned(), PortVisibility::Unspecified),
            ("hardpoint_unnamed".to_owned(), PortVisibility::Unspecified),
        ],
        "the patched tree names none of the Loadout's ports"
    );
    Ok(())
}

#[test]
fn a_vehicle_patch_file_replaces_the_children_it_names_but_not_the_modifications() -> TestResult {
    let tree = resolve(DEFINITION, "FullOverride", Some(&standard_files()))?;
    assert!(tree.issues().is_empty(), "issues: {:?}", tree.issues());
    let vehicle = tree.vehicle().ok_or("no vehicle")?;
    assert_eq!(part_names(vehicle.iter()), ["shell"]);
    Ok(())
}

#[test]
fn a_modification_without_patch_file_falls_back_to_the_file_named_after_the_definition()
-> TestResult {
    let tree = resolve(DEFINITION, "Fallback", Some(&standard_files()))?;
    let vehicle = tree.vehicle().ok_or("no vehicle")?;
    assert_eq!(part_names(vehicle.item_ports()), ["hardpoint_fallback"]);
    assert!(
        vehicle
            .patch_path()
            .is_some_and(|path| path.ends_with("TEST_Ship_Fallback.xml")),
        "patch path"
    );
    assert!(
        vehicle.issues().is_empty(),
        "the Vehicle names no such Modification, a file does"
    );
    Ok(())
}

#[test]
fn a_missing_patch_file_or_modification_is_an_issue_and_the_base_stays() -> TestResult {
    for (modification, reason) in [("Missing", "Nope"), ("Unknown", "no Modification Unknown")] {
        let tree = resolve(DEFINITION, modification, Some(&standard_files()))?;
        let kinds: Vec<_> = tree.issues().iter().map(LoadoutError::kind).collect();
        assert_eq!(kinds, ["vehicle_implementation"], "{modification}");
        assert!(
            tree.issues()
                .iter()
                .any(|issue| issue.to_string().contains(reason)),
            "{modification}: {:?}",
            tree.issues()
        );
        let vehicle = tree.vehicle().ok_or("no vehicle")?;
        assert_eq!(
            part_names(vehicle.item_ports()),
            ["Hardpoint_Gun", "hardpoint_seat"]
        );
        assert_eq!(vehicle.patch_path(), None);
    }
    Ok(())
}

#[test]
fn a_wrapper_with_an_id_no_element_has_replaces_the_child_of_its_name() -> TestResult {
    let patch = r#"<Modifications><Parts id="modParts"><Part name="other" class="Animated"/></Parts></Modifications>"#;
    let base = BASE.replace(
        r#"<Modification name="Patched""#,
        r#"<Modification name="Wrapped" patchFile="Modifications/Wrapped"/><Modification name="Patched""#,
    );
    let files = files(&[
        (DEFINITION, base.as_str()),
        (
            "Scripts/Entities/Vehicles/Implementations/Xml/Modifications/Wrapped.xml",
            patch,
        ),
    ]);
    let vehicle = VehicleImplementation::read(&files, "TEST_Ship", DEFINITION, Some("Wrapped"))?;
    assert!(vehicle.issues().is_empty(), "{:?}", vehicle.issues());
    assert_eq!(part_names(vehicle.iter()), ["other"]);
    let report = vehicle.modification_report().ok_or("no report")?;
    assert_eq!((report.wholesale_children(), report.id_children()), (1, 0));
    assert_eq!(report.unmatched_ids().len(), 0);
    Ok(())
}

#[test]
fn a_wrapper_without_matching_children_is_recorded_as_replaced() -> TestResult {
    let patch = r#"<Modifications><Parts id="modParts"><Part name="other" class="Animated"/></Parts></Modifications>"#;
    let base = BASE.replace(
        r#"<Modification name="Patched""#,
        r#"<Modification name="Wrapped" patchFile="Modifications/Wrapped"/><Modification name="Patched""#,
    );
    let files = files(&[
        (DEFINITION, base.as_str()),
        (
            "Scripts/Entities/Vehicles/Implementations/Xml/Modifications/Wrapped.xml",
            patch,
        ),
    ]);
    let vehicle = VehicleImplementation::read(&files, "TEST_Ship", DEFINITION, Some("Wrapped"))?;
    let report = vehicle.modification_report().ok_or("no report")?;
    assert_eq!(report.replaced_ids(), ["modParts"]);
    Ok(())
}

#[test]
fn a_wrapper_whose_children_match_merges_into_the_base_child_and_keeps_the_rest() -> TestResult {
    let patch = r#"<Modifications><Parts id="modParts">
        <Part id="partGun" name="Hardpoint_Gun" class="ItemPort"><ItemPort id="portGun" flags=""/></Part>
        <Part id="typo" name="x"/>
    </Parts></Modifications>"#;
    let base = BASE.replace(
        r#"<Modification name="Patched""#,
        r#"<Modification name="Wrapped" patchFile="Modifications/Wrapped"/><Modification name="Patched""#,
    );
    let files = files(&[
        (DEFINITION, base.as_str()),
        (
            "Scripts/Entities/Vehicles/Implementations/Xml/Modifications/Wrapped.xml",
            patch,
        ),
    ]);
    let vehicle = VehicleImplementation::read(&files, "TEST_Ship", DEFINITION, Some("Wrapped"))?;
    assert_eq!(
        part_names(vehicle.item_ports()),
        ["Hardpoint_Gun", "hardpoint_seat"],
        "the other parts stay"
    );
    assert_eq!(
        vehicle.port_visibility("hardpoint_gun"),
        PortVisibility::Visible
    );
    let report = vehicle.modification_report().ok_or("no report")?;
    assert_eq!(report.id_children(), 1);
    assert_eq!(report.unmatched_ids(), ["typo"]);
    assert_eq!(report.replaced_ids().len(), 0);
    assert!(
        vehicle
            .issues()
            .iter()
            .any(|issue| issue.to_string().contains("typo")),
        "{:?}",
        vehicle.issues()
    );
    Ok(())
}

#[test]
fn a_wrapper_merges_descendants_at_any_depth_and_leaves_its_root_part_unapplied() -> TestResult {
    let patch = r#"<Modifications><Parts id="modParts">
        <Part name="Other_Root" class="Animated" mass="999999">
          <Parts><Part name="body"><Parts>
            <Part id="partGun" name="Hardpoint_Gun" class="ItemPort"><ItemPort id="portGun" flags=""/></Part>
          </Parts></Part></Parts>
        </Part>
    </Parts></Modifications>"#;
    let base = BASE.replace(
        r#"<Modification name="Patched""#,
        r#"<Modification name="Wrapped" patchFile="Modifications/Wrapped"/><Modification name="Patched""#,
    );
    let files = files(&[
        (DEFINITION, base.as_str()),
        (
            "Scripts/Entities/Vehicles/Implementations/Xml/Modifications/Wrapped.xml",
            patch,
        ),
    ]);
    let vehicle = VehicleImplementation::read(&files, "TEST_Ship", DEFINITION, Some("Wrapped"))?;
    assert_eq!(
        vehicle
            .parts()
            .first()
            .and_then(p4kx_assembly::VehiclePart::mass),
        Some(1500.0),
        "the patch's root part carries no id and is not applied"
    );
    assert_eq!(
        vehicle.port_visibility("hardpoint_gun"),
        PortVisibility::Visible
    );
    assert_eq!(
        part_names(vehicle.item_ports()),
        ["Hardpoint_Gun", "hardpoint_seat"]
    );
    let report = vehicle.modification_report().ok_or("no report")?;
    assert_eq!(report.replaced_ids().len(), 0);
    assert_eq!(report.id_children(), 1);
    Ok(())
}

#[test]
fn an_empty_id_is_no_id_and_never_an_unmatched_one() -> TestResult {
    let patch = r#"<Modifications><Parts id="modParts">
        <Part name="Other_Root" class="Animated"><Animated id=""/>
          <Parts><Part name="body"><Parts>
            <Part id="partGun" name="Hardpoint_Gun" class="ItemPort"><ItemPort id="portGun" flags=""/></Part>
          </Parts></Part></Parts>
        </Part>
    </Parts><Part id="" name="stray"/></Modifications>"#;
    let base = BASE.replace(
        r#"<Modification name="Patched""#,
        r#"<Modification name="Wrapped" patchFile="Modifications/Wrapped"/><Modification name="Patched""#,
    );
    let files = files(&[
        (DEFINITION, base.as_str()),
        (
            "Scripts/Entities/Vehicles/Implementations/Xml/Modifications/Wrapped.xml",
            patch,
        ),
    ]);
    let vehicle = VehicleImplementation::read(&files, "TEST_Ship", DEFINITION, Some("Wrapped"))?;
    let report = vehicle.modification_report().ok_or("no report")?;
    assert_eq!(
        report.unmatched_ids().len(),
        0,
        "{:?}",
        report.unmatched_ids()
    );
    assert_eq!(report.id_children(), 1);
    assert!(
        vehicle
            .issues()
            .iter()
            .all(|issue| !issue.to_string().contains("ids that no element")),
        "{:?}",
        vehicle.issues()
    );
    Ok(())
}
