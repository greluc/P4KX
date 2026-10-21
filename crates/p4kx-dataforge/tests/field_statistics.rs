//! Field statistics over a synthetic DataForge: counting, kinds, empty values, numbers, the
//! struct a strong pointer owns in the path and Record selections.

mod field_common;

use p4kx_dataforge::{DataForge, DataType, FieldKind, FieldStat, FieldStatistics, Number};

use field_common::{TestResult, database, paths, stat};

fn kinds(stat: &FieldStat) -> Vec<String> {
    stat.kinds()
        .iter()
        .map(|(kind, count)| format!("{kind} {count}"))
        .collect()
}

#[test]
fn values_records_and_kinds_are_counted_per_path() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    let statistics = FieldStatistics::of_all(&forge)?;

    assert_eq!(statistics.records(), 4, "Records examined");
    assert_eq!(statistics.unreadable_records(), 0, "unreadable Records");
    assert_eq!(statistics.unreadable_values(), 0, "unreadable values");
    let name = stat(&statistics, "name")?;
    assert_eq!(name.occurrences(), 4, "name occurs in every Record");
    assert_eq!(name.records(), 4, "name Records");
    assert_eq!(kinds(name), ["String 4"], "name kinds");
    let seats = stat(&statistics, "seats")?;
    assert_eq!(
        seats.occurrences(),
        3,
        "one per element, an empty array has none"
    );
    assert_eq!(seats.records(), 2, "only two Records have seats");
    assert_eq!(kinds(seats), ["Int32[] 3"], "an array's elements");
    assert_eq!(
        seats.kinds().first().map(|(kind, _)| *kind),
        Some(FieldKind::new(DataType::Int32, true)),
        "the kind of an element"
    );
    Ok(())
}

#[test]
fn the_struct_a_strong_pointer_owns_is_a_segment_of_the_path() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    let statistics = FieldStatistics::of_all(&forge)?;

    let components = stat(&statistics, "Components")?;
    assert_eq!(components.occurrences(), 4, "pointers");
    assert_eq!(kinds(components), ["StrongPointer[] 4"], "pointer elements");
    assert_eq!(
        stat(&statistics, "Components/WeaponParams")?.occurrences(),
        3,
        "weapons"
    );
    assert_eq!(
        stat(&statistics, "Components/Params")?.occurrences(),
        1,
        "plain params"
    );
    let rate = stat(&statistics, "Components/WeaponParams/fireRate")?;
    assert_eq!(rate.occurrences(), 3, "fire rates");
    assert_eq!(rate.records(), 2, "Records with weapons");
    assert_eq!(rate.min(), Some(Number::Single(-1.5)), "smallest rate");
    assert_eq!(rate.max(), Some(Number::Single(6.0)), "largest rate");
    assert_eq!(rate.distinct(), 3, "distinct rates");
    assert!(
        statistics.get("Components/fireRate").is_none(),
        "the actual struct, not the declared one, names the segment"
    );
    Ok(())
}

#[test]
fn an_inline_struct_adds_its_fields_without_a_type_segment() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    let statistics = FieldStatistics::of_all(&forge)?;

    assert_eq!(
        kinds(stat(&statistics, "position")?),
        ["Class 3"],
        "the struct itself"
    );
    assert_eq!(stat(&statistics, "position/x")?.occurrences(), 3, "x");
    Ok(())
}

#[test]
fn empty_values_are_counted_but_are_not_zero() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    let statistics = FieldStatistics::of_all(&forge)?;

    let name = stat(&statistics, "name")?;
    assert_eq!(name.empty(), 1, "the empty text");
    assert_eq!(name.distinct(), 2, "a and box, the empty text is no value");
    let anchor = stat(&statistics, "anchor")?;
    assert_eq!(
        (anchor.occurrences(), anchor.empty()),
        (3, 3),
        "null pointers"
    );
    assert_eq!(stat(&statistics, "link")?.empty(), 3, "null References");
    assert!(
        statistics.children("anchor").next().is_none(),
        "a null pointer owns no struct"
    );
    let crew = stat(&statistics, "crew")?;
    assert_eq!(crew.empty(), 0, "zero is a value");
    assert_eq!(crew.min(), Some(Number::Integer(-2)), "smallest crew");
    assert_eq!(crew.max(), Some(Number::Integer(4)), "largest crew");
    assert_eq!(crew.distinct(), 3, "0, 4 and -2");
    assert_eq!(stat(&statistics, "name")?.min(), None, "text has no number");
    Ok(())
}

#[test]
fn negative_zero_and_zero_are_one_value() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    let statistics = FieldStatistics::of_all(&forge)?;

    let y = stat(&statistics, "position/y")?;
    assert_eq!(y.distinct(), 1, "-0.0 and 0.0");
    let positive_zero = |number: Option<Number>| matches!(number, Some(Number::Single(value)) if value.is_sign_positive() && value.abs() < f32::MIN_POSITIVE);
    assert!(positive_zero(y.min()), "the smallest is 0.0: {:?}", y.min());
    assert!(positive_zero(y.max()), "the largest is 0.0: {:?}", y.max());
    let mass = stat(&statistics, "mass")?;
    assert_eq!(mass.distinct(), 3, "1.5, 0.0 and 3.0");
    assert!(
        matches!(mass.min(), Some(Number::Double(value)) if value.is_sign_positive()),
        "the smallest mass is 0.0: {:?}",
        mass.min()
    );
    assert_eq!(
        mass.min().map(|number| number.to_string()),
        Some("0".to_owned()),
        "printed without a sign"
    );
    Ok(())
}

#[test]
fn a_selection_counts_only_its_records() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;
    let ships: Vec<u32> = forge
        .path_order()
        .iter()
        .copied()
        .filter(|&index| {
            forge
                .record(index)
                .and_then(|record| record.record_path().ok())
                .is_some_and(|path| path.starts_with("ships/t"))
        })
        .collect();

    let statistics = FieldStatistics::of_records(&forge, &ships)?;

    assert_eq!(ships.len(), 2, "two and three");
    assert_eq!(statistics.records(), 2, "examined");
    assert_eq!(stat(&statistics, "crew")?.occurrences(), 2, "crew");
    assert_eq!(
        stat(&statistics, "crew")?.min(),
        Some(Number::Integer(-2)),
        "min"
    );
    assert_eq!(
        stat(&statistics, "crew")?.max(),
        Some(Number::Integer(4)),
        "max"
    );
    assert_eq!(
        stat(&statistics, "name")?.distinct(),
        1,
        "the crate is not selected"
    );
    Ok(())
}

#[test]
fn paths_are_sorted_and_children_list_one_level() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    let statistics = FieldStatistics::of_all(&forge)?;

    let names: Vec<&str> = statistics.children("").map(FieldStat::path).collect();
    assert_eq!(
        names,
        [
            "anchor",
            "Components",
            "crew",
            "link",
            "mass",
            "name",
            "position",
            "seats"
        ],
        "root fields ignoring case"
    );
    let below: Vec<&str> = statistics
        .children("Components")
        .map(FieldStat::path)
        .collect();
    assert_eq!(
        below,
        ["Components/Params", "Components/WeaponParams"],
        "the structs below a pointer array"
    );
    let all = paths(&statistics);
    let mut sorted = all.clone();
    sorted.sort_by_key(|path| path.to_ascii_lowercase());
    assert_eq!(all.first(), Some(&"anchor"), "first path");
    assert_eq!(all.len(), sorted.len(), "no path twice");
    Ok(())
}
