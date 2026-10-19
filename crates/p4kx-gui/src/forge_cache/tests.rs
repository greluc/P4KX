use std::error::Error;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
};

use super::ReadFailure;
use crate::{Language, Model};

type TestResult = Result<(), Box<dyn Error>>;

fn write_archive(path: &Path, extra: bool) -> TestResult {
    let mut forge = DataForgeBuilder::new()
        .structure(TestStruct::new("Maker").field("rank", TestType::Int32))
        .record(
            TestRecord::new(1, "Maker", "Maker.One", "libs/makers/one.xml")
                .value("rank", TestValue::Int32(7)),
        );
    if extra {
        forge = forge.record(TestRecord::new(
            2,
            "Maker",
            "Maker.Two",
            "libs/makers/two.xml",
        ));
    }
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", forge.build()))
        .write_to(path)?;
    Ok(())
}

fn opened(path: &Path) -> Result<Model, Box<dyn Error>> {
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path.to_path_buf(), || {});
    let deadline = Instant::now() + Duration::from_secs(30);
    while model.browser().is_none() {
        if Instant::now() > deadline {
            return Err("the Archive did not open".into());
        }
        thread::sleep(Duration::from_millis(5));
        model.poll();
    }
    Ok(model)
}

#[test]
fn checking_the_archive_does_not_wait_for_a_read_of_dataforge_and_the_read_is_dropped() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    write_archive(&path, false)?;
    let mut model = opened(&path)?;
    let index = std::sync::Arc::clone(model.browser().ok_or("no Archive")?.shared_index());
    let (started, running) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();

    let (checked, took, after, again) = thread::scope(|scope| -> Result<_, Box<dyn Error>> {
        let reader_index = &index;
        let reader = scope.spawn(move || {
            reader_index.forge().read(reader_index, || {
                let _ = started.send(());
                let _ = released.recv();
            })
        });
        running.recv_timeout(Duration::from_secs(30))?;
        write_archive(&path, true)?;
        let watchdog = release.clone();
        scope.spawn(move || {
            thread::sleep(Duration::from_secs(3));
            let _ = watchdog.send(());
        });
        let begun = Instant::now();
        let checked = model.check_archive();
        let took = begun.elapsed();
        let _ = release.send(());
        let read = reader.join().map_err(|_| "the reader panicked")?;
        let after = (read, index.forge().is_read());
        let again = (index.forge().read(&index, || {}), index.forge().reads());
        Ok((checked, took, after, again))
    })?;

    assert!(checked, "the change is noticed");
    assert!(
        took < Duration::from_secs(2),
        "check_archive waited for the read: {took:?}"
    );
    assert!(after.0.is_err(), "the read is dropped: {:?}", after.0);
    assert!(!after.1, "nothing is cached");
    assert!(
        matches!(again.0, Err(ReadFailure::ArchiveChanged(_))),
        "the next read stops at the changed Archive: {:?}",
        again.0
    );
    assert_eq!(again.1, 2, "the next read is counted");
    Ok(())
}
