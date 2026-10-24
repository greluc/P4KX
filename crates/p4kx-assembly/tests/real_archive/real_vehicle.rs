// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Vehicle implementations and their Modifications in a real Archive: ships and ground vehicles,
//! the issues the reader records, the patch shapes of ADR-0050 and some known vehicles.
//!
//! Run with `--nocapture` to see the summary; it holds counts, paths and class names only.

use std::collections::BTreeMap;
use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ENTITY_TYPE, EntityNode, LoadoutResolver, MAX_XML_ELEMENT_DEPTH,
    MAX_XML_LOADOUT_LEN, PortVisibility, VehicleImplementation,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

use crate::real_vehicle_support;

use real_vehicle_support::{depth, file_len, flipped_ports, mass, nodes, port_names};

type TestResult = Result<(), Box<dyn Error>>;

const SPACESHIPS: &str = "libs/foundry/records/entities/spaceships/";
const GROUND_VEHICLES: &str = "libs/foundry/records/entities/groundvehicles/";

/// A gap in the game data the reader reports and the test accepts: issues of vehicles whose class
/// name starts with `class`, of the kind `kind`, whose message holds `text`.
struct Allowed {
    class: &'static str,
    kind: &'static str,
    text: &'static str,
    reason: &'static str,
}

const ALLOWED_ISSUES: &[Allowed] = &[Allowed {
    class: "MISC_Starfarer_Wreck",
    kind: "vehicle_implementation",
    text: "there is no such file",
    reason: "the Modifications of the wrecked Starfarer name patch files MISC_Starfarer_Dead*.xml, which the Archive does not hold",
}];

fn allowed(class: &str, kind: &str, message: &str) -> bool {
    ALLOWED_ISSUES.iter().any(|allowed| {
        !allowed.reason.is_empty()
            && class.starts_with(allowed.class)
            && kind == allowed.kind
            && message.contains(allowed.text)
    })
}

/// How many children of a patch file are of each shape: `Vehicle` child, id-less, with an id.
#[derive(Debug, Default, Clone, Copy)]
struct Shape {
    vehicle: usize,
    wholesale: usize,
    id: usize,
}

#[derive(Debug, Default)]
struct Tally {
    vehicles: usize,
    ground_vehicles: usize,
    with_treads_or_wheels: usize,
    patched: usize,
    unmatched_ids: usize,
    unmatched_children: usize,
    unmatched_elems: usize,
    ports: usize,
    unspecified_ports: usize,
    deepest: usize,
    largest_file: usize,
    issues: Vec<String>,
    unexpected: Vec<String>,
    shapes: BTreeMap<String, Shape>,
    users: BTreeMap<String, Vec<String>>,
    differs: BTreeMap<String, bool>,
    changes: Vec<String>,
    misses: Vec<String>,
    harbinger: Option<(bool, f64)>,
    flips: Vec<String>,
    replaced: BTreeMap<String, Vec<String>>,
}

fn patches<'a>(tally: &'a Tally, needle: &str) -> Vec<(&'a str, Shape)> {
    tally
        .users
        .keys()
        .filter(|patch| patch.contains(needle))
        .map(|patch| {
            (
                patch.as_str(),
                tally.shapes.get(patch).copied().unwrap_or_default(),
            )
        })
        .collect()
}

fn note_report(
    tally: &mut Tally,
    class_name: &str,
    patch: &str,
    base: &VehicleImplementation,
    vehicle: &VehicleImplementation,
) {
    if let Some(report) = vehicle.modification_report() {
        let shape = tally.shapes.entry(patch.to_owned()).or_default();
        shape.vehicle = shape.vehicle.max(report.vehicle_children());
        shape.wholesale = shape.wholesale.max(report.wholesale_children());
        shape.id = shape.id.max(report.id_children());
        tally.unmatched_ids += report.unmatched_ids().len();
        tally.unmatched_children += report.unmatched_children().len();
        tally.unmatched_elems += report.unmatched_elems().len();
        if !report.replaced_ids().is_empty() {
            tally
                .replaced
                .insert(class_name.to_owned(), report.replaced_ids().to_vec());
        }
        if !report.unmatched_elems().is_empty() {
            let known: Vec<&str> = base.ids().collect();
            let in_base: Vec<&str> = report
                .unmatched_elems()
                .iter()
                .map(String::as_str)
                .filter(|id| known.contains(id))
                .collect();
            tally.misses.push(format!(
                "{class_name}: {}; in the pre-patch base: {}",
                report.unmatched_elems().join(", "),
                in_base.join(", ")
            ));
        }
    }
}

fn note_patch(
    tally: &mut Tally,
    files: &ArchiveFiles<'_>,
    class_name: &str,
    vehicle: &VehicleImplementation,
) -> Result<(), Box<dyn Error>> {
    let (Some(modification), Some(patch)) = (vehicle.modification(), vehicle.patch_path()) else {
        return Ok(());
    };
    let patch = patch.to_ascii_lowercase();
    tally.largest_file = tally.largest_file.max(file_len(files, &patch));
    tally.patched += 1;
    tally
        .users
        .entry(patch.clone())
        .or_default()
        .push(class_name.to_owned());
    let base = VehicleImplementation::read(files, class_name, vehicle.path(), None)?;
    note_report(tally, class_name, &patch, &base, vehicle);
    if class_name == "AEGS_Vanguard_Harbinger" {
        let ids: Vec<&str> = vehicle.ids().collect();
        tally.harbinger = Some((
            ids.contains(&"modFlightController") && ids.contains(&"modName"),
            mass(vehicle) / mass(&base),
        ));
    }
    let flips = flipped_ports(&base, vehicle);
    if !flips.is_empty() {
        tally.flips.push(format!(
            "{class_name} ({modification}): {} ports: {}",
            flips.len(),
            flips.join(", ")
        ));
    }
    tally.differs.insert(
        class_name.to_owned(),
        port_names(&base) != port_names(vehicle) || base.iter().count() != vehicle.iter().count(),
    );
    tally.changes.push(format!(
        "{class_name} ({modification}) parts {}->{} mass {:.0}->{:.0} ports {}->{}",
        base.iter().count(),
        vehicle.iter().count(),
        mass(&base),
        mass(vehicle),
        base.item_ports().count(),
        vehicle.item_ports().count()
    ));
    Ok(())
}

fn collect(
    tally: &mut Tally,
    files: &ArchiveFiles<'_>,
    tree: &EntityNode,
    ground: bool,
) -> Result<(), Box<dyn Error>> {
    for node in nodes(tree) {
        for issue in node
            .issues()
            .iter()
            .filter(|issue| issue.kind() == "vehicle_implementation")
        {
            let line = format!("{}: {issue}", node.class_name());
            if !allowed(node.class_name(), issue.kind(), &issue.to_string()) {
                tally.unexpected.push(line.clone());
            }
            tally.issues.push(line);
        }
    }
    let Some(vehicle) = tree.vehicle() else {
        return Ok(());
    };
    tally.vehicles += 1;
    tally.ground_vehicles += usize::from(ground);
    tally.with_treads_or_wheels +=
        usize::from(ground && vehicle.treads().chain(vehicle.wheels()).next().is_some());
    tally.ports += tree.ports().len();
    tally.unspecified_ports += tree
        .ports()
        .iter()
        .filter(|port| port.visibility() == PortVisibility::Unspecified)
        .count();
    tally.deepest = tally.deepest.max(depth(vehicle.parts()));
    tally.largest_file = tally.largest_file.max(file_len(files, vehicle.path()));
    note_patch(tally, files, tree.class_name(), vehicle)
}

#[expect(
    clippy::print_stdout,
    reason = "the summary of the run is the measurement"
)]
fn report(tally: &Tally) {
    println!(
        "vehicles {} (ground {}, with treads or wheels {}), patched {}, unmatched ids/children/elems {}/{}/{}",
        tally.vehicles,
        tally.ground_vehicles,
        tally.with_treads_or_wheels,
        tally.patched,
        tally.unmatched_ids,
        tally.unmatched_children,
        tally.unmatched_elems
    );
    println!(
        "ports {} of which unspecified {}, deepest part nesting {} of {MAX_XML_ELEMENT_DEPTH}, largest file {} of {MAX_XML_LOADOUT_LEN} bytes, issues {}",
        tally.ports,
        tally.unspecified_ports,
        tally.deepest,
        tally.largest_file,
        tally.issues.len()
    );
    for issue in &tally.issues {
        println!("issue {issue}");
    }
    for (patch, classes) in &tally.users {
        let shape = tally.shapes.get(patch).copied().unwrap_or_default();
        println!(
            "patch {patch} shapes vehicle/id-less/id {}/{}/{}: {}",
            shape.vehicle,
            shape.wholesale,
            shape.id,
            classes.join(", ")
        );
    }
    for change in &tally.changes {
        println!("{change}");
    }
    for miss in &tally.misses {
        println!("unmatched Elems {miss}");
    }
    for flip in &tally.flips {
        println!("visibility flips {flip}");
    }
    for (class, ids) in &tally.replaced {
        println!("wrapper replaced wholesale {class}: {}", ids.join(", "));
    }
}

fn assert_known_vehicles(tally: &Tally, jump: &EntityNode) {
    assert!(
        !tally.flips.is_empty(),
        "no Modification flips the visibility of a port"
    );
    let (kept, ratio) = tally.harbinger.unwrap_or((false, 0.0));
    assert!(
        kept,
        "the Harbinger keeps modFlightController and modName after the patch"
    );
    assert!(
        (0.5..=2.0).contains(&ratio),
        "the Harbinger's mass stays plausible: {ratio:.2} of the base"
    );
    assert!(
        tally
            .replaced
            .get("AEGS_Vanguard_Harbinger")
            .is_none_or(|ids| !ids.iter().any(|id| id == "modParts")),
        "the Harbinger wrapper is merged by id, not replaced wholesale"
    );
    assert!(
        nodes(jump)
            .iter()
            .any(|node| !node.landing_gear().is_empty()),
        "ORIG_890Jump lists no landing gear"
    );
    let f7cm = tally
        .users
        .iter()
        .find(|(patch, _)| patch.ends_with("modifications/anvl_hornet_f7cm.xml"))
        .map(|(_, classes)| classes);
    let f7cm = f7cm.map_or(&[][..], Vec::as_slice);
    assert!(
        f7cm.len() >= 2,
        "the F7CM variants share one patch file: {f7cm:?}"
    );
    assert!(
        f7cm.iter()
            .all(|class| tally.differs.get(class) == Some(&true)),
        "the F7CM variants differ from the base: {f7cm:?}"
    );
    let sentinel = patches(tally, "sentinel");
    assert!(
        sentinel.iter().any(|(_, shape)| shape.vehicle > 0),
        "the Sentinel is a full override: {sentinel:?}"
    );
    let r350 = patches(tally, "350r");
    assert!(
        r350.iter()
            .any(|(_, shape)| shape.id > 0 && shape.vehicle == 0),
        "the 350r patches by id: {r350:?}"
    );
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_vehicle_implementations_of_a_real_archive_read_and_patch_as_decided() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let mut tally = Tally::default();
    for record in forge.records() {
        let path = record
            .record_path()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let ground = path.starts_with(GROUND_VEHICLES);
        if !(ground || path.starts_with(SPACESHIPS))
            || record.type_name().is_ok_and(|name| name != ENTITY_TYPE)
        {
            continue;
        }
        let tree = resolver.resolve(&record)?;
        collect(&mut tally, &files, &tree, ground)?;
    }
    let jump = resolver.entities().find("ORIG_890Jump")?;
    let jump = resolver.resolve(&jump)?;
    report(&tally);

    assert!(
        tally.unexpected.is_empty(),
        "issues: {:?}",
        tally.unexpected
    );
    assert!(tally.vehicles > 50, "vehicles: {}", tally.vehicles);
    assert!(tally.patched > 0, "no Modification applied a patch file");
    assert!(
        tally.deepest < MAX_XML_ELEMENT_DEPTH,
        "deepest: {}",
        tally.deepest
    );
    assert!(
        u64::try_from(tally.largest_file)? <= MAX_XML_LOADOUT_LEN,
        "largest file: {}",
        tally.largest_file
    );
    assert!(
        tally.unspecified_ports < tally.ports,
        "no Loadout port got a visibility: {} of {}",
        tally.unspecified_ports,
        tally.ports
    );
    assert!(
        tally.with_treads_or_wheels > 0,
        "no ground vehicle has treads or wheels"
    );
    assert_known_vehicles(&tally, &jump);
    Ok(())
}
