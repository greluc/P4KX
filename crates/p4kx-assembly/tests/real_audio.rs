//! The audio index of a real Archive and the audio triggers of its ships.

use std::error::Error;

use p4kx_assembly::audio::{AudioIndex, wwise_id};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_triggers_of_every_ship_mostly_reach_a_soundbank() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let index = AudioIndex::build(&archive);
    assert!(
        index.triggers().len() > 10_000,
        "{} triggers",
        index.triggers().len()
    );
    assert!(
        index.soundbanks().len() > 1_000,
        "{} soundbanks",
        index.soundbanks().len()
    );
    assert!(
        index.issues().len() * 100 < index.soundbanks().len(),
        "few unreadable Entries: {:?}",
        index.issues()
    );
    let named_by_hash = index
        .soundbanks()
        .iter()
        .filter(|bank| {
            bank.file_name()
                .strip_suffix(".bnk")
                .is_some_and(|stem| wwise_id(stem) == bank.id())
        })
        .count();
    assert!(
        named_by_hash * 10 > index.soundbanks().len() * 9,
        "soundbank IDs are the Wwise IDs of their names: {named_by_hash} of {}",
        index.soundbanks().len()
    );
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let (mut triggers, mut matched) = (0_usize, 0_usize);
    for record in forge.records().filter(|record| {
        record
            .type_name()
            .is_ok_and(|name| name == "EntityClassDefinition")
            && record
                .record_path()
                .is_ok_and(|path| path.contains("/entities/spaceships/"))
    }) {
        let audio = index.entity_audio(&record)?;
        triggers += audio.triggers().len();
        matched += audio.matched_count();
    }
    assert!(triggers > 5_000, "{triggers} triggers of ships");
    assert!(
        matched * 100 >= triggers * 95,
        "{matched} of {triggers} ship triggers reach a soundbank"
    );
    Ok(())
}
