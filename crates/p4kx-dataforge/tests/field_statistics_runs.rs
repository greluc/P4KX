//! Field statistics over a synthetic DataForge: the cap of distinct values, the same result on any
//! number of threads, a stopped computation and indexes that are no Record.

mod field_common;

use std::error::Error;

use p4kx_dataforge::{DataForge, FieldStatistics, MAX_DISTINCT_VALUES, Number};
use p4kx_testkit::TestValue;

use field_common::{TestResult, database, plain, schema, ship, stat, weapon};

#[test]
fn distinct_values_stop_at_the_cap() -> TestResult {
    let mut builder = schema();
    for index in 0..200_u32 {
        builder = builder.record(
            ship(u128::from(index) + 1, &format!("ships/s{index:03}.xml"))
                .value("crew", TestValue::Int32(i32::try_from(index)?)),
        );
    }
    let bytes = builder.build();
    let forge = DataForge::open(&bytes)?;

    let statistics = FieldStatistics::of_all(&forge)?;

    let crew = stat(&statistics, "crew")?;
    assert_eq!(crew.distinct(), MAX_DISTINCT_VALUES, "at the cap");
    assert!(crew.distinct_capped(), "more than the cap");
    assert_eq!(crew.max(), Some(Number::Integer(199)), "extremes are exact");
    Ok(())
}

#[test]
fn the_result_does_not_depend_on_the_number_of_threads() -> TestResult {
    let mut builder = schema();
    for index in 0..1_500_u32 {
        let rate = f32::from(u16::try_from(index % 97)?) / 4.0;
        let record = ship(u128::from(index) + 1, &format!("ships/s{index:04}.xml"))
            .value("crew", TestValue::Int32(i32::try_from(index % 13)?))
            .value("name", TestValue::text(format!("ship {}", index % 50)))
            .value("Components", TestValue::Array(vec![weapon(rate), plain()]));
        builder = builder.record(record);
    }
    let bytes = builder.build();
    let forge = DataForge::open(&bytes)?;

    let run = |threads: usize| -> Result<FieldStatistics, Box<dyn Error>> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()?;
        Ok(pool.install(|| FieldStatistics::of_all(&forge))?)
    };
    let one = run(1)?;
    let four = run(4)?;
    let again = run(4)?;

    assert_eq!(one, four, "one thread and four");
    assert_eq!(four, again, "two runs");
    assert_eq!(
        stat(&one, "Components/WeaponParams")?.occurrences(),
        1_500,
        "weapons"
    );
    assert_eq!(
        stat(&one, "name")?.distinct(),
        MAX_DISTINCT_VALUES,
        "names are capped"
    );
    Ok(())
}

#[test]
fn a_stopped_computation_gives_nothing() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    let stopped = FieldStatistics::of_records_while(&forge, forge.path_order(), || false)?;
    let finished = FieldStatistics::of_records_while(&forge, forge.path_order(), || true)?;

    assert!(stopped.is_none(), "stopped before the first Record");
    assert_eq!(
        finished.map(|statistics| statistics.records()),
        Some(4),
        "finished"
    );
    Ok(())
}

#[test]
fn an_index_that_is_no_record_is_unreadable() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    let statistics = FieldStatistics::of_records(&forge, &[0, 9_999])?;

    assert_eq!(statistics.records(), 2, "examined");
    assert_eq!(statistics.unreadable_records(), 1, "the missing Record");
    Ok(())
}
