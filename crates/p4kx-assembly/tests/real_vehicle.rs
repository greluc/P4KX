//! Vehicle implementations and their Modifications in a real Archive: ships and ground vehicles,
//! the issues the reader records, the patch shapes of ADR-0050 and some known vehicles.
//!
//! Run with `--nocapture` to see the summary; it holds counts, paths and class names only.

use std::collections::BTreeMap;
use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, Attachment, ENTITY_TYPE, EntityNode, LoadoutFiles, LoadoutResolver,
    MAX_XML_ELEMENT_DEPTH, MAX_XML_LOADOUT_LEN, PortVisibility, VehicleImplementation, VehiclePart,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SPACESHIPS: &str = "libs/foundry/records/entities/spaceships/";
const GROUND_VEHICLES: &str = "libs/foundry/records/entities/groundvehicles/";

/// The issues the reader may record on the live Archive; every other issue fails the test.
const ALLOWED_ISSUES: &[&str] = &[];

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
    shapes: BTreeMap<String, Shape>,
    users: BTreeMap<String, Vec<String>>,
    differs: BTreeMap<String, bool>,
    changes: Vec<String>,
}

fn nodes(root: &EntityNode) -> Vec<&EntityNode> {
    let mut all = vec![root];
    all.extend(
        root.walk()
            .into_iter()
            .filter_map(|(_, port)| match port.attachment() {
                Attachment::Entity(child) => Some(child.as_ref()),
                _ => None,
            }),
    );
    all
}

fn depth(parts: &[VehiclePart]) -> usize {
    parts
        .iter()
        .map(|part| depth(part.children()).saturating_add(1))
        .max()
        .unwrap_or(0)
}

fn mass(vehicle: &VehicleImplementation) -> f64 {
    vehicle.iter().filter_map(VehiclePart::mass).sum()
}

fn port_names(vehicle: &VehicleImplementation) -> Vec<&str> {
    vehicle.item_ports().map(VehiclePart::name).collect()
}

fn file_len(files: &ArchiveFiles<'_>, path: &str) -> usize {
    files.read(path).map_or(0, |bytes| bytes.len())
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
    if let Some(report) = vehicle.modification_report() {
        let shape = tally.shapes.entry(patch).or_default();
        shape.vehicle = shape.vehicle.max(report.vehicle_children());
        shape.wholesale = shape.wholesale.max(report.wholesale_children());
        shape.id = shape.id.max(report.id_children());
        tally.unmatched_ids += report.unmatched_ids().len();
        tally.unmatched_children += report.unmatched_children().len();
        tally.unmatched_elems += report.unmatched_elems().len();
    }
    let base = VehicleImplementation::read(files, class_name, vehicle.path(), None)?;
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
        tally.issues.extend(
            node.issues()
                .iter()
                .filter(|issue| issue.kind() == "vehicle_implementation")
                .map(|issue| format!("{}: {issue}", node.class_name())),
        );
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

fn assert_known_vehicles(tally: &Tally, jump: &EntityNode) {
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
            .any(|(_, shape)| shape.wholesale > 0 && shape.vehicle == 0 && shape.id == 0),
        "the 350r is parts-only: {r350:?}"
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

    let unexpected: Vec<_> = tally
        .issues
        .iter()
        .filter(|issue| !ALLOWED_ISSUES.iter().any(|allowed| issue.contains(allowed)))
        .collect();
    assert!(unexpected.is_empty(), "issues: {unexpected:?}");
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
