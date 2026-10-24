// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Searching the triggers and events of the audio index, and the Entities that name a trigger.

use std::error::Error;

use p4kx_assembly::audio::{AudioIndex, MAX_SEARCH_PAGE, TriggerUsers, Unmatched};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestAtl, TestBank, TestEntity, TestEntry, TestHircObject,
    audio_schema, entity_schema, wwise_short_id,
};

type TestResult = Result<(), Box<dyn Error>>;

const PLAY: u16 = 0x0403;

fn bank(events: &[&str]) -> Vec<u8> {
    events
        .iter()
        .zip(1_u32..)
        .fold(TestBank::new(0xB0), |bank, (event, action)| {
            bank.object(TestHircObject::action(action, PLAY, 0x100 + action))
                .object(TestHircObject::event(wwise_short_id(event), &[action]))
        })
        .bytes()
}

fn with_archive(check: impl FnOnce(&Archive) -> TestResult) -> TestResult {
    let entity = |guid, class: &str, triggers: &[&str]| {
        triggers.iter().fold(
            TestEntity::new(guid, class, format!("entities/{class}.xml")),
            |entity, trigger| entity.audio_trigger(*trigger),
        )
    };
    let database = [
        entity(1, "TEST_A", &["Play_Engine", "Play_Missing"]),
        entity(2, "TEST_B", &["play_engine", "Play_Engine", "Play_Door"]),
        entity(3, "TEST_C", &[]),
    ]
    .iter()
    .fold(
        audio_schema(entity_schema(DataForgeBuilder::new())),
        |builder, entity| builder.record(entity.record()),
    );
    let atl = TestAtl::new("SC")
        .trigger("Play_Engine", &["Play_Engine_Loop", "Play_Hum"])
        .trigger("Play_Door", &["Play_Door_Open"])
        .trigger("Play_Quiet", &[])
        .trigger("Stop_Hum", &["Stop_Hum_All"]);
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database.build()))
        .entry(TestEntry::zstd(
            "Data\\Libs\\GameAudio\\SC.xml",
            atl.to_cryxml(),
        ))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\Ship.bnk",
            bank(&["Play_Engine_Loop", "Play_Door_Open"]),
        ))
        .write_to(&path)?;
    check(&Archive::open_with_key(
        path,
        Some(p4kx_testkit::test_key()),
    )?)
}

fn names(index: &AudioIndex, text: &str, offset: usize, limit: usize) -> (Vec<String>, usize) {
    let page = index.search(text, offset, limit);
    (
        page.hits()
            .iter()
            .map(|hit| hit.trigger().name().to_owned())
            .collect(),
        page.total(),
    )
}

#[test]
fn a_substring_matches_trigger_and_event_names_ignoring_case() -> TestResult {
    with_archive(|archive| {
        let index = AudioIndex::build(archive);
        assert_eq!(
            names(&index, "HUM", 0, 10),
            (vec!["Play_Engine".to_owned(), "Stop_Hum".to_owned()], 2)
        );
        assert_eq!(names(&index, "door", 0, 10).1, 1);
        assert_eq!(names(&index, "", 0, 10).1, 4, "an empty search matches all");
        Ok(())
    })
}

#[test]
fn a_glob_matches_the_whole_name() -> TestResult {
    with_archive(|archive| {
        let index = AudioIndex::build(archive);
        assert_eq!(names(&index, "play_*_open", 0, 10).0, ["Play_Door"]);
        assert_eq!(names(&index, "play_d?or", 0, 10).0, ["Play_Door"]);
        assert_eq!(
            names(&index, "play_d", 0, 10).1,
            1,
            "a substring matches inside"
        );
        assert_eq!(
            names(&index, "d?or", 0, 10).1,
            0,
            "a glob matches the whole name"
        );
        Ok(())
    })
}

#[test]
fn pages_are_bounded_and_deterministic() -> TestResult {
    with_archive(|archive| {
        let index = AudioIndex::build(archive);
        let first = index.search("", 0, 3);
        assert_eq!(first.hits().len(), 3);
        assert_eq!(first.total(), 4);
        assert_eq!(first.next_offset(), Some(3));
        let second = index.search("", 3, 3);
        assert_eq!(second.hits().len(), 1);
        assert_eq!(second.next_offset(), None);
        let again = index.search("", 0, 3);
        let order = |page: &p4kx_assembly::audio::SearchPage<'_>| -> Vec<String> {
            page.hits()
                .iter()
                .map(|hit| hit.trigger().name().to_owned())
                .collect()
        };
        assert_eq!(order(&first), order(&again));
        assert!(index.search("", 0, usize::MAX).hits().len() <= MAX_SEARCH_PAGE);
        assert!(index.search("", 99, 3).hits().is_empty());
        Ok(())
    })
}

#[test]
fn a_hit_lists_its_events_soundbanks_and_what_matched() -> TestResult {
    with_archive(|archive| {
        let index = AudioIndex::build(archive);
        let page = index.search("hum", 0, 10);
        let engine = page.hits().first().ok_or("a hit")?;
        assert!(!engine.matched_by_trigger());
        assert_eq!(engine.unmatched(), None);
        let events: Vec<(String, usize, bool)> = engine
            .events()
            .map(|(event, matched)| {
                (
                    event.event().name().to_owned(),
                    event.soundbanks().count(),
                    matched,
                )
            })
            .collect();
        assert_eq!(
            events,
            [
                ("Play_Engine_Loop".to_owned(), 1, false),
                ("Play_Hum".to_owned(), 0, true),
            ]
        );
        let stop = page.hits().get(1).ok_or("a second hit")?;
        assert_eq!(stop.unmatched(), Some(Unmatched::NoSoundbank));
        Ok(())
    })
}

#[test]
fn the_entities_of_a_trigger_are_counted_once_each_in_definition_order() -> TestResult {
    with_archive(|archive| {
        let entry = DataForgeEntry::read(archive)?;
        let forge = entry.open()?;
        let index = AudioIndex::build(archive);
        let users = TriggerUsers::build(&forge, &|name| index.trigger(name).is_some());
        assert_eq!(users.entity_count(), 3);
        assert_eq!(users.unreadable_records(), 0);
        assert_eq!(users.count("PLAY_ENGINE"), 2, "each Entity counts once");
        let classes: Vec<String> = users
            .entities(&forge, "play_engine")
            .filter_map(|record| record.name().ok().map(str::to_owned))
            .collect();
        assert_eq!(
            classes,
            [
                "EntityClassDefinition.TEST_A",
                "EntityClassDefinition.TEST_B"
            ]
        );
        assert_eq!(users.count("Play_Door"), 1);
        assert_eq!(
            users.count("Play_Missing"),
            0,
            "undefined triggers are not kept"
        );
        assert_eq!(users.count("Play_Quiet"), 0);
        let only_doors = TriggerUsers::build(&forge, &|name| name == "play_door");
        assert_eq!(
            only_doors.count("Play_Engine"),
            0,
            "only wanted triggers are kept"
        );
        assert_eq!(only_doors.count("Play_Door"), 1);
        Ok(())
    })
}
