//! Resolution bounds the depth, the fan-out and the size of an untrusted soundbank hierarchy.

use std::error::Error;

use p4kx_assembly::audio::{
    ArchiveAudio, BreakReason, MAX_CONTAINER_CHILDREN, MAX_EVENT_OBJECTS, MAX_PATH_DEPTH,
    TriggerSounds,
};
use p4kx_extraction::Archive;
use p4kx_testkit::{
    ArchiveBuilder, TestAtl, TestBank, TestContainerKind, TestEntry, TestHircObject, TestNodeBase,
    wwise_short_id,
};

type TestResult = Result<(), Box<dyn Error>>;

const PLAY: u16 = 0x0403;

fn resolve(bank: TestBank, check: impl FnOnce(&TriggerSounds<'_>) -> TestResult) -> TestResult {
    let bytes = bank
        .object(TestHircObject::event(wwise_short_id("Play_It"), &[1]))
        .object(TestHircObject::action(1, PLAY, 10))
        .bytes();
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored(
            "Data\\Libs\\GameAudio\\SC.xml",
            TestAtl::new("SC").trigger("Play_It", &["Play_It"]).to_xml(),
        ))
        .entry(TestEntry::stored("Data\\Sounds\\wwise\\Deep.bnk", bytes))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;
    let audio = ArchiveAudio::new(&archive);
    let sounds = audio.resolve_trigger("Play_It").ok_or("Play_It resolves")?;
    check(&sounds)
}

fn random(id: u32, children: &[u32]) -> TestHircObject {
    TestHircObject::container(TestContainerKind::Random, id, children, TestNodeBase::Full)
}

#[test]
fn a_path_deeper_than_the_limit_is_cut_at_the_limit() -> TestResult {
    let bank = (10..60).fold(TestBank::new(1), |bank, id| {
        bank.object(random(id, &[id + 1]))
    });
    resolve(bank, |sounds| {
        assert!(sounds.variants().is_empty(), "no variants");
        let [deep] = sounds.breaks() else {
            return Err(format!("one break: {:?}", sounds.breaks()).into());
        };
        assert_eq!(
            deep.reason(),
            &BreakReason::TooDeep {
                limit: MAX_PATH_DEPTH
            },
            "reason"
        );
        assert_eq!(deep.path().len(), MAX_PATH_DEPTH, "path at the limit");
        Ok(())
    })
}

#[test]
fn children_beyond_the_fan_out_limit_are_left_out_and_reported() -> TestResult {
    let children = vec![20; MAX_CONTAINER_CHILDREN + 5];
    let bank = TestBank::new(1)
        .object(random(10, &children))
        .object(TestHircObject::sound(20, 7, 2, 0));
    resolve(bank, |sounds| {
        assert_eq!(
            sounds.variants().len(),
            MAX_CONTAINER_CHILDREN,
            "the first children"
        );
        let [wide] = sounds.breaks() else {
            return Err(format!("one break: {:?}", sounds.breaks()).into());
        };
        assert_eq!(
            (wide.reason(), wide.object().map(|step| step.id)),
            (
                &BreakReason::TooManyChildren {
                    count: MAX_CONTAINER_CHILDREN + 5,
                    limit: MAX_CONTAINER_CHILDREN,
                },
                Some(10)
            ),
            "reported at the container"
        );
        Ok(())
    })
}

#[test]
fn a_hierarchy_leading_to_more_objects_than_the_budget_is_cut_once() -> TestResult {
    let bank = TestBank::new(1)
        .object(random(10, &[11; 40]))
        .object(random(11, &[12; 40]))
        .object(random(12, &[20; 40]))
        .object(TestHircObject::sound(20, 7, 2, 0));
    resolve(bank, |sounds| {
        let [budget] = sounds.breaks() else {
            return Err(format!("one break: {:?}", sounds.breaks()).into());
        };
        assert_eq!(
            budget.reason(),
            &BreakReason::TooManyObjects {
                limit: MAX_EVENT_OBJECTS
            },
            "reason"
        );
        assert!(
            sounds.variants().len() < MAX_EVENT_OBJECTS,
            "{} variants",
            sounds.variants().len()
        );
        Ok(())
    })
}
