//! The audio index is built while rayon tasks that need it wait, without a deadlock.

use std::error::Error;
use std::sync::mpsc;
use std::time::Duration;

use p4kx_assembly::audio::ArchiveAudio;
use p4kx_extraction::Archive;
use p4kx_testkit::{ArchiveBuilder, TestAtl, TestBank, TestEntry, TestHircObject, wwise_short_id};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const SOUNDBANKS: u32 = 3_000;
const TASKS: usize = 256;
const ROUNDS: usize = 40;
const DEADLINE: Duration = Duration::from_secs(120);

fn bank(number: u32) -> Vec<u8> {
    TestBank::new(number)
        .object(TestHircObject::event(number, &[1]))
        .bytes()
}

fn lookups(archive: &Archive) -> usize {
    (0..ROUNDS)
        .map(|_| {
            let audio = ArchiveAudio::new(archive);
            (0..TASKS)
                .into_par_iter()
                .filter(|_| audio.index().trigger("Play_It").is_some())
                .count()
        })
        .sum()
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "stress test: too slow to hit the race without optimisation, run it with --release"
)]
fn the_audio_index_builds_while_rayon_tasks_wait_for_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    let mut builder = ArchiveBuilder::new().entry(TestEntry::stored(
        "Data\\Libs\\GameAudio\\SC.xml",
        TestAtl::new("SC").trigger("Play_It", &["Play_It"]).to_xml(),
    ));
    for number in 0..SOUNDBANKS {
        builder = builder.entry(TestEntry::stored(
            format!("Data\\Sounds\\wwise\\Bank{number}.bnk"),
            bank(wwise_short_id(&format!("Bank{number}"))),
        ));
    }
    builder.write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;

    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let _sent = done.send(lookups(&archive));
    });
    let found = finished
        .recv_timeout(DEADLINE)
        .map_err(|_| "the lookups did not finish: the audio index deadlocked")?;

    assert_eq!(found, ROUNDS * TASKS, "every task finds the trigger");
    Ok(())
}
