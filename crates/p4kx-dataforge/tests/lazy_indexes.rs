//! The lazily built Record indexes stay usable from many rayon tasks at once.

use std::error::Error;
use std::sync::mpsc;
use std::time::Duration;

use p4kx_dataforge::DataForge;
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const RECORDS: u32 = 200_000;
const ROUNDS: usize = 40;
const DEADLINE: Duration = Duration::from_secs(120);

fn database() -> Vec<u8> {
    (0..RECORDS)
        .fold(
            DataForgeBuilder::new()
                .structure(TestStruct::new("Ship").field("speed", TestType::Int32)),
            |builder, index| {
                builder.record(TestRecord::new(
                    (u128::from(RECORDS - index) << 64) | u128::from(index),
                    "Ship",
                    format!("Ship.{index}"),
                    format!("libs/ships/ship_{}.xml", RECORDS - index),
                ))
            },
        )
        .build()
}

fn look_up_every_record(bytes: &[u8]) -> Result<usize, String> {
    let forge = DataForge::open(bytes).map_err(|error| error.to_string())?;
    Ok(forge
        .records()
        .par_bridge()
        .filter(|record| {
            forge.record_by_guid(record.guid()).is_some()
                && record
                    .record_path()
                    .is_ok_and(|path| forge.records_at_path(path).next().is_some())
        })
        .count())
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "stress test: too slow to hit the race without optimisation, run it with --release"
)]
fn the_record_indexes_build_while_rayon_tasks_wait_for_them() -> TestResult {
    let bytes = database();
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let found = (0..ROUNDS)
            .map(|_| look_up_every_record(&bytes))
            .collect::<Result<Vec<_>, _>>();
        let _sent = done.send(found);
    });

    let found = finished
        .recv_timeout(DEADLINE)
        .map_err(|_| "the lookups did not finish: a lazy index deadlocked")??;

    assert!(
        found.iter().all(|&count| count == RECORDS as usize),
        "every lookup finds its Record"
    );
    Ok(())
}
