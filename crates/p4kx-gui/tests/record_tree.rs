//! The virtual Record tree of DataForge in the path tree, driven without drawing.

use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use p4kx_gui::{
    Browser, DataForgeFailure, Language, Model, RecordInfo, RecordNodeKind, RecordTree,
    RecordTreeStatus,
};
use p4kx_testkit::{ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

fn database() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(TestStruct::new("EntityClassDefinition"))
        .structure(TestStruct::new("AmmoParams"))
        .enumeration("ItemType", ["Ship", "Weapon"])
        .record(TestRecord::new(
            1,
            "EntityClassDefinition",
            "EntityClassDefinition.AEGS_Avenger",
            "libs/foundry/records/entities/spaceships/aegs_avenger.xml",
        ))
        .record(TestRecord::new(
            2,
            "AmmoParams",
            "AmmoParams.Laser_S1",
            "libs/foundry/records/ammoparams/ammo.xml",
        ))
        .record(TestRecord::new(
            3,
            "EntityClassDefinition",
            "EntityClassDefinition.ANVL_Arrow",
            "libs/foundry/records/entities/spaceships/anvl_arrow.xml",
        ))
        .record(TestRecord::new(
            4,
            "AmmoParams",
            "AmmoParams.Ballistic_S1",
            "libs/foundry/records/ammoparams/ammo.xml",
        ))
}

fn with_dataforge(name: &str, bytes: Vec<u8>) -> ArchiveBuilder {
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", "config"))
        .entry(TestEntry::zstd(name, bytes))
        .entry(TestEntry::stored("readme.txt", "readme"))
}

fn opened(builder: &ArchiveBuilder) -> Result<(TempDir, PathBuf, Model), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    builder.write_to(&path)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path.clone(), || {});
    wait_until(&mut model, |model| model.browser().is_some())?;
    Ok((dir, path, model))
}

fn wait_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !done(model) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
        if let Some(browser) = model.browser_mut() {
            browser.poll();
        }
    }
    Ok(())
}

fn records(model: &Model) -> Result<&RecordTree, Box<dyn Error>> {
    model
        .browser()
        .and_then(Browser::records)
        .ok_or_else(|| "no DataForge node".into())
}

fn settled(model: &Model) -> bool {
    model
        .browser()
        .and_then(Browser::records)
        .is_some_and(|tree| !matches!(tree.status(), RecordTreeStatus::Loading))
}

fn expand(model: &mut Model, path: &str) -> TestResult {
    let tree = records(model)?;
    let node = tree.find(path).ok_or_else(|| format!("no node {path}"))?;
    if !tree.is_expanded(node) {
        model
            .browser_mut()
            .ok_or("no Archive")?
            .toggle_record_node(node, || {});
    }
    wait_until(model, settled)
}

fn labels(tree: &RecordTree) -> Vec<String> {
    tree.rows()
        .iter()
        .map(|row| {
            let name = if row.node == tree.root() {
                "DataForge"
            } else {
                tree.node(row.node).name()
            };
            format!("{}{name}", "  ".repeat(row.depth))
        })
        .collect()
}

#[test]
fn the_node_appears_for_game2_and_falls_back_to_game() -> TestResult {
    let (_a, _, game2) = opened(&with_dataforge("Data\\Game2.dcb", database().build()))?;
    let (_b, _, game) = opened(&with_dataforge("Data\\Game.dcb", database().build()))?;
    let (_c, _, none) = opened(&with_dataforge("Data\\Game3.dcb", database().build()))?;

    assert_eq!(records(&game2)?.entry_path(), "Data\\Game2.dcb", "Game2");
    assert_eq!(records(&game)?.entry_path(), "Data\\Game.dcb", "Game");
    assert!(
        none.browser().and_then(Browser::records).is_none(),
        "no node without DataForge"
    );
    Ok(())
}

#[test]
fn nothing_is_loaded_until_the_node_is_opened() -> TestResult {
    let (_dir, _, model) = opened(&with_dataforge("Data\\Game2.dcb", database().build()))?;

    let tree = records(&model)?;

    assert!(
        matches!(tree.status(), RecordTreeStatus::NotLoaded),
        "not loaded"
    );
    assert_eq!(labels(tree), ["DataForge"], "the node alone");
    Ok(())
}

#[test]
fn opening_the_node_loads_in_the_background() -> TestResult {
    let (_dir, _, mut model) = opened(&with_dataforge("Data\\Game2.dcb", database().build()))?;
    let root = records(&model)?.root();

    model
        .browser_mut()
        .ok_or("no Archive")?
        .toggle_record_node(root, || {});

    assert!(
        matches!(records(&model)?.status(), RecordTreeStatus::Loading),
        "loading until polled"
    );
    wait_until(&mut model, settled)?;
    assert_eq!(labels(records(&model)?), ["DataForge", "  libs"], "loaded");
    Ok(())
}

#[test]
fn records_are_grouped_by_record_path() -> TestResult {
    let (_dir, _, mut model) = opened(&with_dataforge("Data\\Game2.dcb", database().build()))?;

    for path in [
        "",
        "libs",
        "libs/foundry",
        "libs/foundry/records",
        "libs/foundry/records/ammoparams",
        "libs/foundry/records/ammoparams/ammo",
        "libs/foundry/records/entities",
        "libs/foundry/records/entities/spaceships",
    ] {
        expand(&mut model, path)?;
    }

    let tree = records(&model)?;
    assert_eq!(
        labels(tree),
        [
            "DataForge",
            "  libs",
            "    foundry",
            "      records",
            "        ammoparams",
            "          ammo",
            "            AmmoParams.Ballistic_S1",
            "            AmmoParams.Laser_S1",
            "        entities",
            "          spaceships",
            "            aegs_avenger",
            "            anvl_arrow",
        ],
        "folders, a record folder and Records"
    );
    let kind = |path: &str| tree.find(path).map(|node| tree.node(node).kind());
    assert_eq!(
        kind("libs/foundry/records/ammoparams/ammo"),
        Some(RecordNodeKind::RecordFolder),
        "two Records share a record path"
    );
    assert!(
        matches!(
            kind("libs/foundry/records/entities/spaceships/aegs_avenger"),
            Some(RecordNodeKind::Record(_))
        ),
        "a Record alone at its record path"
    );
    Ok(())
}

#[test]
fn a_record_shows_its_name_type_and_guid() -> TestResult {
    let (_dir, _, mut model) = opened(&with_dataforge("Data\\Game2.dcb", database().build()))?;
    expand(&mut model, "")?;

    let tree = records(&model)?;
    let node = tree
        .find("libs/foundry/records/entities/spaceships/anvl_arrow")
        .ok_or("no Record")?;
    let RecordNodeKind::Record(record) = tree.node(node).kind() else {
        return Err("not a Record".into());
    };
    let record = tree.record(record).ok_or("no Record data")?;

    assert_eq!(record.name(), "EntityClassDefinition.ANVL_Arrow", "name");
    assert_eq!(record.type_name(), "EntityClassDefinition", "type");
    assert_eq!(
        record.record_path(),
        "libs/foundry/records/entities/spaceships/anvl_arrow",
        "record path"
    );
    assert_eq!(
        record.guid().to_string(),
        "00000000-0000-0000-0000-000000000003",
        "GUID"
    );
    Ok(())
}

#[test]
fn the_summary_counts_records_structs_and_enums() -> TestResult {
    let (_dir, _, mut model) = opened(&with_dataforge("Data\\Game2.dcb", database().build()))?;
    expand(&mut model, "")?;

    let RecordTreeStatus::Ready(summary) = records(&model)?.status() else {
        return Err("not loaded".into());
    };

    assert_eq!(summary.version(), 6, "version");
    assert_eq!(summary.record_count(), 4, "Records");
    assert_eq!(summary.struct_count(), 2, "structs");
    assert_eq!(summary.enum_count(), 1, "enums");
    let first: Vec<&str> = summary
        .first_records()
        .iter()
        .map(RecordInfo::name)
        .collect();
    assert_eq!(
        first,
        [
            "EntityClassDefinition.AEGS_Avenger",
            "AmmoParams.Laser_S1",
            "EntityClassDefinition.ANVL_Arrow",
            "AmmoParams.Ballistic_S1",
        ],
        "in definition order"
    );
    Ok(())
}

#[test]
fn the_loaded_dataforge_is_kept_until_the_archive_is_reloaded() -> TestResult {
    let (_dir, _, mut model) = opened(&with_dataforge("Data\\Game2.dcb", database().build()))?;
    expand(&mut model, "")?;
    let root = records(&model)?.root();

    let browser = model.browser_mut().ok_or("no Archive")?;
    browser.toggle_record_node(root, || {});
    browser.toggle_record_node(root, || {});
    assert!(
        matches!(records(&model)?.status(), RecordTreeStatus::Ready(_)),
        "not loaded again"
    );

    model.reload(|| {});
    wait_until(&mut model, |model| model.opening_path().is_none())?;
    assert!(
        matches!(records(&model)?.status(), RecordTreeStatus::NotLoaded),
        "dropped on reload"
    );
    Ok(())
}

#[test]
fn a_change_on_disk_drops_the_loaded_dataforge() -> TestResult {
    let (_dir, path, mut model) = opened(&with_dataforge("Data\\Game2.dcb", database().build()))?;
    expand(&mut model, "")?;

    with_dataforge("Data\\Game.dcb", database().build()).write_to(&path)?;
    model.check_archive();

    let tree = records(&model)?;
    assert!(
        matches!(tree.status(), RecordTreeStatus::ArchiveChanged),
        "dropped"
    );
    assert_eq!(labels(tree), ["DataForge"], "no Records shown");
    Ok(())
}

#[test]
fn an_unknown_version_fails_with_its_number() -> TestResult {
    let bytes = database().version(9).build();
    let (_dir, _, mut model) = opened(&with_dataforge("Data\\Game2.dcb", bytes))?;

    expand(&mut model, "")?;

    assert!(
        matches!(
            records(&model)?.status(),
            RecordTreeStatus::Failed(DataForgeFailure::Unsupported { version: 9 })
        ),
        "version 9"
    );
    Ok(())
}

#[test]
fn a_corrupt_dataforge_fails_with_the_reason() -> TestResult {
    let (_dir, _, mut model) = opened(&with_dataforge("Data\\Game2.dcb", vec![6, 0, 0]))?;

    expand(&mut model, "")?;

    let RecordTreeStatus::Failed(DataForgeFailure::Unreadable(reason)) = records(&model)?.status()
    else {
        return Err("no failure".into());
    };
    assert!(reason.contains("truncated"), "{reason}");
    Ok(())
}

#[test]
fn selecting_a_folder_ends_the_record_selection() -> TestResult {
    let (_dir, _, mut model) = opened(&with_dataforge("Data\\Game2.dcb", database().build()))?;
    let browser = model.browser_mut().ok_or("no Archive")?;
    let root = browser.records().ok_or("no node")?.root();

    browser.select_record_node(root, || {});
    let selected = browser.records().and_then(RecordTree::selected);
    let folder = browser.index().find_folder("Data").ok_or("no folder")?;
    browser.select_folder(folder);

    assert_eq!(selected, Some(root), "the node was selected");
    assert_eq!(
        browser.records().and_then(RecordTree::selected),
        None,
        "and is no more"
    );
    wait_until(&mut model, settled)
}

#[test]
fn searching_ends_the_record_selection() -> TestResult {
    let (_dir, _, mut model) = opened(&with_dataforge("Data\\Game2.dcb", database().build()))?;
    let browser = model.browser_mut().ok_or("no Archive")?;
    let root = browser.records().ok_or("no node")?.root();

    browser.select_record_node(root, || {});
    browser.search("config", || {});

    assert_eq!(
        browser.records().and_then(RecordTree::selected),
        None,
        "the search results are shown instead"
    );
    wait_until(&mut model, settled)
}
