//! Invalid indices and corrupt nesting become error values at their place; the Record still builds.

use std::error::Error;

use p4kx_dataforge::{
    DataForge, DataForgeError, Field, MAX_NESTING_DEPTH, MAX_RECORD_VALUES, RecordTree, Value,
    ValueError,
};
use p4kx_testkit::{
    DataForgeBuilder, TestInstance, TestPool, TestRecord, TestStruct, TestType, TestValue,
};

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLE: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;

fn raw(words: [u32; 2]) -> TestValue {
    TestValue::Raw(words.iter().flat_map(|word| word.to_le_bytes()).collect())
}

fn value<'t, 'f>(tree: &'t RecordTree<'f>, name: &str) -> Result<&'t Value<'f>, String> {
    tree.fields()
        .iter()
        .find(|field| field.name() == name)
        .map(Field::value)
        .ok_or_else(|| format!("no field {name}"))
}

fn node() -> TestStruct {
    TestStruct::new("Node")
        .field("next", TestType::StrongPointer("Node".to_owned()))
        .field("mark", TestType::Int8)
}

fn holder() -> Vec<u8> {
    DataForgeBuilder::new()
        .structure(node())
        .structure(
            TestStruct::new("Holder")
                .field("bad_struct", TestType::StrongPointer("Node".to_owned()))
                .field("bad_instance", TestType::StrongPointer("Node".to_owned()))
                .field("bad_weak", TestType::WeakPointer("Node".to_owned()))
                .array("numbers", TestType::Int32)
                .array("nodes", TestType::Class("Node".to_owned()))
                .array("unknown", TestType::Class("Missing".to_owned()))
                .array("huge", TestType::Int8)
                .field("after", TestType::Int8),
        )
        .values(TestPool::Int32, 2)
        .record(
            TestRecord::new(SAMPLE, "Holder", "Holder.One", "holder.xml")
                .value("bad_struct", raw([99, 0]))
                .value("bad_instance", raw([0, 7]))
                .value("bad_weak", raw([1, u32::MAX - 1]))
                .value("numbers", raw([3, 1]))
                .value("nodes", raw([1, 0]))
                .value("huge", raw([u32::MAX, 0]))
                .value("after", TestValue::Int8(5)),
        )
        .build()
}

fn holder_errors(bytes: &[u8]) -> Result<Vec<(String, ValueError)>, Box<dyn Error>> {
    let forge = DataForge::open(bytes)?;
    let tree = forge.records().next().ok_or("no Record")?.tree()?;
    assert_eq!(
        value(&tree, "after")?,
        &Value::Int8(5),
        "the rest still builds"
    );
    Ok(tree
        .errors()
        .map(|(name, error)| (name.to_owned(), *error))
        .collect())
}

#[test]
fn pointers_out_of_range_are_errors_at_their_field() -> TestResult {
    let errors = holder_errors(&holder())?;

    assert_eq!(
        errors.get(..3).unwrap_or_default(),
        [
            (
                "bad_struct",
                ValueError::InvalidPointer {
                    struct_index: 99,
                    instance_index: 0
                }
            ),
            (
                "bad_instance",
                ValueError::InvalidPointer {
                    struct_index: 0,
                    instance_index: 7
                }
            ),
            (
                "bad_weak",
                ValueError::InvalidPointer {
                    struct_index: 1,
                    instance_index: u32::MAX - 1
                }
            ),
        ]
        .map(|(name, error)| (name.to_owned(), error)),
        "strong and weak pointers"
    );
    Ok(())
}

#[test]
fn arrays_out_of_range_are_errors_at_their_field() -> TestResult {
    let errors = holder_errors(&holder())?;

    assert_eq!(
        errors.get(3..).unwrap_or_default(),
        [
            (
                "numbers",
                ValueError::ArrayOutOfRange {
                    first: 1,
                    count: 3,
                    len: 2,
                    pool: "int32 pool"
                }
            ),
            (
                "nodes",
                ValueError::ArrayOutOfRange {
                    first: 0,
                    count: 1,
                    len: 0,
                    pool: "instance block"
                }
            ),
            ("unknown", ValueError::UnknownStruct { index: u16::MAX }),
            (
                "huge",
                ValueError::ArrayOutOfRange {
                    first: 0,
                    count: u32::MAX,
                    len: 0,
                    pool: "int8 pool"
                }
            ),
        ]
        .map(|(name, error)| (name.to_owned(), error)),
        "arrays outside their pool or of an unknown struct"
    );
    Ok(())
}

#[test]
fn an_instance_owned_twice_is_an_error_and_ends_a_cycle() -> TestResult {
    let bytes = DataForgeBuilder::new()
        .structure(node())
        .instance(
            "a",
            TestInstance::new("Node").value("next", TestValue::pointer("b")),
        )
        .instance(
            "b",
            TestInstance::new("Node")
                .value("next", TestValue::pointer("a"))
                .value("mark", TestValue::Int8(2)),
        )
        .record(
            TestRecord::new(SAMPLE, "Node", "Node.One", "node.xml")
                .value("next", TestValue::pointer("a"))
                .value("mark", TestValue::Int8(1)),
        )
        .build();
    let forge = DataForge::open(&bytes)?;

    let tree = forge.records().next().ok_or("no Record")?.tree()?;

    let errors: Vec<_> = tree.errors().map(|(name, error)| (name, *error)).collect();
    assert_eq!(
        errors,
        [(
            "next",
            ValueError::OwnedTwice {
                struct_index: 0,
                instance_index: 1
            }
        )],
        "the loop back to a is cut, nested errors are counted"
    );
    let located: Vec<_> = tree
        .located_errors()
        .map(|(location, _)| location.to_string())
        .collect();
    assert_eq!(
        located,
        ["next.next.next"],
        "the full location of the nested error"
    );
    assert_eq!(tree.error_count(), 1, "counted without collecting");
    assert_eq!(value(&tree, "mark")?, &Value::Int8(1), "the Record builds");
    Ok(())
}

#[test]
fn owned_structs_nested_too_deep_are_errors() -> TestResult {
    let mut chain = TestValue::Null;
    for _ in 0..MAX_NESTING_DEPTH + 5 {
        chain = TestValue::Struct(TestInstance::new("Node").value("next", chain));
    }
    let bytes = DataForgeBuilder::new()
        .structure(node())
        .record(TestRecord::new(SAMPLE, "Node", "Node.One", "node.xml").value("next", chain))
        .build();
    let forge = DataForge::open(&bytes)?;

    let tree = forge.records().next().ok_or("no Record")?.tree()?;

    let mut depth = 0;
    let mut current = value(&tree, "next")?;
    while let Value::Struct(structure) = current {
        depth += 1;
        current = structure.fields().first().map(Field::value).ok_or("next")?;
    }
    assert_eq!(depth, MAX_NESTING_DEPTH, "nested structs below the Record");
    assert_eq!(
        current,
        &Value::Error(ValueError::TooDeep {
            limit: MAX_NESTING_DEPTH
        }),
        "the next level is an error"
    );
    Ok(())
}

#[test]
fn a_struct_that_contains_itself_inline_stops_at_the_value_limit() -> TestResult {
    let bytes = DataForgeBuilder::new()
        .structure(
            TestStruct::new("Loop")
                .field("left", TestType::Class("Loop".to_owned()))
                .field("right", TestType::Class("Loop".to_owned()))
                .declared_size(0),
        )
        .record(TestRecord::new(SAMPLE, "Loop", "Loop.One", "loop.xml"))
        .build();
    let forge = DataForge::open(&bytes)?;

    let result = forge.records().next().ok_or("no Record")?.tree();

    assert!(
        matches!(
            result,
            Err(DataForgeError::RecordTooLarge {
                limit: MAX_RECORD_VALUES
            })
        ),
        "bounded: {result:?}"
    );
    Ok(())
}

#[test]
fn an_array_larger_than_the_value_limit_is_an_error_at_its_field() -> TestResult {
    let count = u32::try_from(MAX_RECORD_VALUES + 1)?;
    let bytes = DataForgeBuilder::new()
        .structure(TestStruct::new("Big").array("bytes", TestType::Int8))
        .values(TestPool::Int8, count)
        .record(
            TestRecord::new(SAMPLE, "Big", "Big.One", "big.xml").value("bytes", raw([count, 0])),
        )
        .build();
    let forge = DataForge::open(&bytes)?;

    let tree = forge.records().next().ok_or("no Record")?.tree()?;

    assert_eq!(
        value(&tree, "bytes")?,
        &Value::Error(ValueError::TooManyValues {
            count,
            limit: MAX_RECORD_VALUES
        }),
        "rejected before it is read"
    );
    let located: Vec<_> = tree
        .located_errors()
        .map(|(location, _)| location.to_string())
        .collect();
    assert_eq!(located, ["bytes"], "the field itself");
    Ok(())
}

#[test]
fn error_values_of_nested_data_carry_a_reason() -> TestResult {
    let reasons = [
        (
            ValueError::InvalidPointer {
                struct_index: 9,
                instance_index: 4,
            },
            "the pointer refers to instance 4 of struct 9, which does not exist",
        ),
        (
            ValueError::ArrayOutOfRange {
                first: 1,
                count: 3,
                len: 2,
                pool: "int32 pool",
            },
            "the array of 3 values from index 1 lies outside the int32 pool of 2 values",
        ),
        (
            ValueError::OwnedTwice {
                struct_index: 0,
                instance_index: 1,
            },
            "instance 1 of struct 0 is owned more than once in the Record",
        ),
        (
            ValueError::TooDeep { limit: 64 },
            "the value is nested deeper than 64 structs",
        ),
        (
            ValueError::DanglingReference {
                guid: "00000001-0002-0003-0004-000000000005".parse()?,
            },
            "the Reference refers to the Record 00000001-0002-0003-0004-000000000005, which does not exist",
        ),
        (
            ValueError::OutsideRecord {
                struct_index: 2,
                instance_index: 3,
            },
            "the weak pointer refers to instance 3 of struct 2, which lies outside the Record",
        ),
        (
            ValueError::TooManyValues { count: 9, limit: 8 },
            "the array of 9 values is larger than the 8 values a Record may hold",
        ),
        (
            ValueError::UnknownStruct { index: 7 },
            "the field refers to struct 7, which does not exist",
        ),
    ];
    for (error, reason) in reasons {
        assert_eq!(error.to_string(), reason, "{error:?}");
    }
    Ok(())
}
