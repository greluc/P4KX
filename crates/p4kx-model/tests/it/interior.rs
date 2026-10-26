// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reading object container descriptions and the `IncludedObjects` of `.soc` files.

use p4kx_model::{
    IncludedObjects, IncludedObjectsError, MAX_STREAMING_DEPTH, ObjectContainer,
    ObjectContainerError, Transform,
};
use p4kx_testkit::{SOC_IDENTITY, TestChunkFile, TestObjectContainer, TestSoc};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn close(actual: [f32; 4], expected: [f32; 4]) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (a - e).abs() < 1e-5)
}

#[test]
fn nested_containers_are_read_from_text_and_from_cryxml() -> TestResult {
    let description = TestObjectContainer::new("ship")
        .child("cockpit", [1.0, 2.0, 3.0], [1.0, 0.0, 0.0, 0.0])
        .child("rear", [0.0, -4.0, 0.0], [0.0, 0.0, 0.0, 1.0])
        .stream("ship.soc");

    for bytes in [description.xml().into_bytes(), description.cryxml()] {
        let container = ObjectContainer::parse(&bytes)?;

        assert_eq!(container.name, "ship");
        assert_eq!(container.streams, ["ship.soc"]);
        assert_eq!(container.children.len(), 2);
        let cockpit = &container.children[0];
        assert_eq!(cockpit.name, "cockpit");
        assert_eq!(cockpit.position, [1.0, 2.0, 3.0]);
        assert!(cockpit.external);
        let turned = container.children[1].transform().ok_or("no transform")?;
        assert!(close(turned.rows[0], [-1.0, 0.0, 0.0, 0.0]), "{turned:?}");
        assert!(close(turned.rows[1], [0.0, -1.0, 0.0, -4.0]), "{turned:?}");
    }
    Ok(())
}

#[test]
fn the_declared_bounds_are_read_and_malformed_ones_are_none() -> TestResult {
    let declared = TestObjectContainer::new("c").bounds_text("-1.5,-2,-3", "4,5.5,6e0");
    let container = ObjectContainer::parse(declared.xml().as_bytes())?;

    let bounds = container.bounds.ok_or("no bounds")?;
    assert_eq!(
        (bounds.min, bounds.max),
        ([-1.5, -2.0, -3.0], [4.0, 5.5, 6.0])
    );
    assert!(bounds.contains([4.0, 5.5, -3.0], 0.0));
    assert!(!bounds.contains([4.1, 0.0, 0.0], 0.0));
    assert!(bounds.contains([4.1, 0.0, 0.0], 0.2));

    for (min, max) in [("1,2", "3,4,5"), ("a,b,c", "3,4,5"), ("inf,0,0", "1,1,1")] {
        let broken = TestObjectContainer::new("c").bounds_text(min, max);
        assert_eq!(
            ObjectContainer::parse(broken.xml().as_bytes())?.bounds,
            None
        );
    }
    let none = TestObjectContainer::new("c");
    assert_eq!(ObjectContainer::parse(none.xml().as_bytes())?.bounds, None);
    Ok(())
}

#[test]
fn streamed_files_nest_and_keep_their_order() -> TestResult {
    let xml = r#"<ObjectContainer name="c"><StreamingOCs>
        <OC name="c.soc"><StreamingOCs><OC name="c/a.soc"/><OC name="c/b.soc"/></StreamingOCs></OC>
    </StreamingOCs></ObjectContainer>"#;

    let container = ObjectContainer::parse(xml.as_bytes())?;

    assert_eq!(container.streams, ["c.soc", "c/a.soc", "c/b.soc"]);
    Ok(())
}

#[test]
fn a_placement_that_is_no_numbers_is_marked_and_a_zero_rotation_has_no_transform() -> TestResult {
    let description = TestObjectContainer::new("c")
        .raw_child(&[("name", "a"), ("pos", "1,two,3")])
        .raw_child(&[("name", "b"), ("rot", "0,0,0,0")])
        .raw_child(&[("name", "c"), ("pos", "inf,0,0")]);

    let container = ObjectContainer::parse(description.xml().as_bytes())?;

    assert!(container.children[0].malformed);
    assert!(!container.children[1].malformed);
    assert!(container.children[1].transform().is_none());
    assert!(container.children[2].malformed || container.children[2].transform().is_none());
    Ok(())
}

#[test]
fn a_file_that_is_no_object_container_is_an_error() {
    assert!(matches!(
        ObjectContainer::parse(b"<Material/>"),
        Err(ObjectContainerError::NotContainer { .. })
    ));
    assert!(matches!(
        ObjectContainer::parse(b"<ObjectContainer"),
        Err(ObjectContainerError::Xml(_))
    ));
    assert!(matches!(
        ObjectContainer::parse(b"CryXmlB\0 truncated"),
        Err(ObjectContainerError::CryXml(_))
    ));
}

#[test]
fn streams_nested_beyond_the_limit_are_refused() {
    let depth = MAX_STREAMING_DEPTH + 1;
    let xml = format!(
        "<ObjectContainer>{}{}</ObjectContainer>",
        "<StreamingOCs><OC name=\"x.soc\">".repeat(depth),
        "</OC></StreamingOCs>".repeat(depth)
    );

    assert!(matches!(
        ObjectContainer::parse(xml.as_bytes()),
        Err(ObjectContainerError::TooMany { .. })
    ));
}

#[test]
fn included_objects_hold_the_models_with_their_transforms() -> TestResult {
    let moved = [
        [1.0, 0.0, 0.0, 1.5],
        [0.0, 1.0, 0.0, -2.0],
        [0.0, 0.0, 1.0, 3.25],
    ];
    let soc = TestSoc::new()
        .model("objects/a.cgf")
        .model("objects/b.cgf")
        .palette("Libs/Foundry/Records/TintPalettes/default")
        .object(1, moved)
        .object_with(0, SOC_IDENTITY, 0);

    let objects = IncludedObjects::read(&soc.bytes())?.ok_or("no chunk")?;

    assert_eq!(objects.models, ["objects/a.cgf", "objects/b.cgf"]);
    assert_eq!(objects.palettes.len(), 1);
    assert_eq!(objects.objects.len(), 2);
    assert_eq!(objects.objects[0].model, 1);
    assert_eq!(objects.objects[0].palette, Some(0));
    assert!(close(
        objects.objects[0].transform.rows[2],
        [0.0, 0.0, 1.0, 3.25]
    ));
    assert!(objects.objects[1].transform.is_identity());
    assert_eq!((objects.invalid, objects.unsupported), (0, None));
    Ok(())
}

#[test]
fn objects_that_cannot_be_placed_are_counted_and_other_kinds_end_the_list() -> TestResult {
    let broken = [[f64::NAN, 0.0, 0.0, 0.0], [0.0; 4], [0.0; 4]];
    let soc = TestSoc::new()
        .model("objects/a.cgf")
        .object(0, SOC_IDENTITY)
        .object(7, SOC_IDENTITY)
        .object(0, broken)
        .object_of_kind(10)
        .object(0, SOC_IDENTITY);

    let objects = IncludedObjects::read(&soc.bytes())?.ok_or("no chunk")?;

    assert_eq!(objects.objects.len(), 1);
    assert_eq!(
        objects.invalid, 2,
        "an unlisted model and a transform that is not finite"
    );
    assert_eq!(objects.unsupported, Some(10));
    Ok(())
}

const fn placed_at(x: f64) -> [[f64; 4]; 3] {
    [
        [1.0, 0.0, 0.0, x],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ]
}

#[test]
fn every_known_layout_reads_all_its_records_at_its_own_length() -> TestResult {
    for layout in [8, 9, 10, 12, 13, 14, 15] {
        let soc = TestSoc::new()
            .layout(layout)
            .model("objects/a.cgf")
            .object(0, placed_at(1.0))
            .object(0, placed_at(2.0))
            .object(0, placed_at(3.0));

        let objects = IncludedObjects::read(&soc.bytes())?.ok_or("no chunk")?;

        let xs: Vec<f32> = objects
            .objects
            .iter()
            .map(|object| object.transform.translation()[0])
            .collect();
        assert_eq!(xs, [1.0, 2.0, 3.0], "layout {layout}");
        assert_eq!((objects.invalid, objects.unsupported), (0, None));
    }
    Ok(())
}

#[test]
fn a_layout_that_is_not_known_is_an_error_unless_the_table_is_empty() -> TestResult {
    let with_object = TestSoc::new()
        .layout(11)
        .model("objects/a.cgf")
        .object(0, SOC_IDENTITY);
    assert!(matches!(
        IncludedObjects::read(&with_object.bytes()),
        Err(IncludedObjectsError::RecordLayout { layout: 11 })
    ));

    let empty = TestSoc::new().layout(11).model("objects/a.cgf");
    let objects = IncludedObjects::read(&empty.bytes())?.ok_or("no chunk")?;
    assert_eq!(objects.objects, []);
    Ok(())
}

#[test]
fn records_of_kind_16_are_skipped_and_the_list_goes_on() -> TestResult {
    for layout in [8, 15] {
        let soc = TestSoc::new()
            .layout(layout)
            .model("objects/a.cgf")
            .object(0, placed_at(1.0))
            .skipped_object()
            .skipped_object()
            .object(0, placed_at(2.0));

        let objects = IncludedObjects::read(&soc.bytes())?.ok_or("no chunk")?;

        assert_eq!(objects.objects.len(), 2, "layout {layout}");
        assert_eq!(objects.objects[1].transform.translation()[0], 2.0);
        assert_eq!(objects.skipped, 2);
        assert_eq!((objects.invalid, objects.unsupported), (0, None));
    }
    Ok(())
}

#[test]
fn a_cut_off_skipped_record_is_counted_not_an_error() -> TestResult {
    let mut chunk = TestSoc::new()
        .model("a.cgf")
        .object(0, SOC_IDENTITY)
        .skipped_object()
        .chunk();
    let at = chunk.len() - 136 - 208 - 4;
    chunk[at..at + 4].copy_from_slice(&(208_u32 + 100).to_le_bytes());
    chunk.truncate(chunk.len() - 36);
    let bytes = TestChunkFile::crch().chunk(0x0010, 1, chunk).bytes();

    let objects = IncludedObjects::read(&bytes)?.ok_or("no chunk")?;

    assert_eq!((objects.objects.len(), objects.skipped), (1, 0));
    assert_eq!(objects.invalid, 1);
    Ok(())
}

#[test]
fn placements_that_are_no_rotation_with_a_scale_are_invalid() -> TestResult {
    let scaled = [
        [0.01, 0.0, 0.0, 5.0],
        [0.0, 0.02, 0.0, 0.0],
        [0.0, 0.0, 300.0, 0.0],
    ];
    let turned = [
        [0.0, -2.0, 0.0, 0.0],
        [2.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, -2.0, 0.0],
    ];
    let singular = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
    ];
    let repeated = [
        [1.0, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ];
    let sheared = [
        [1.0, 0.0, 0.0, 0.0],
        [1.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ];
    let half = std::f64::consts::FRAC_1_SQRT_2;
    let rotated_scaled = [
        [half, -3.0 * half, 0.0, 0.0],
        [half, 3.0 * half, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ];
    let mirrored = [
        [-1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ];
    let soc = TestSoc::new()
        .model("objects/a.cgf")
        .object(0, scaled)
        .object(0, turned)
        .object(0, singular)
        .object(0, repeated)
        .object(0, sheared)
        .object(0, rotated_scaled)
        .object(0, mirrored);

    let objects = IncludedObjects::read(&soc.bytes())?.ok_or("no chunk")?;

    assert_eq!(
        objects.objects.len(),
        4,
        "a scale, a turn, both and a mirror"
    );
    assert_eq!(objects.invalid, 3);
    Ok(())
}

#[test]
fn a_soc_without_included_objects_has_none() -> TestResult {
    let bytes = TestChunkFile::crch().chunk(0x0002, 5, vec![0; 4]).bytes();

    assert_eq!(IncludedObjects::read(&bytes)?, None);
    Ok(())
}

#[test]
fn truncated_and_oversized_tables_are_errors_not_panics() {
    let chunk = TestSoc::new()
        .model("a.cgf")
        .object(0, SOC_IDENTITY)
        .chunk();
    for cut in [0, 7, 100, 300, chunk.len() - 1] {
        let bytes = TestChunkFile::crch()
            .chunk(0x0010, 1, chunk[..cut].to_vec())
            .bytes();
        assert!(IncludedObjects::read(&bytes).is_err(), "cut at {cut}");
    }
    let mut huge = chunk;
    huge[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    let bytes = TestChunkFile::crch().chunk(0x0010, 1, huge).bytes();
    assert!(matches!(
        IncludedObjects::read(&bytes),
        Err(IncludedObjectsError::TooMany { .. })
    ));
    assert!(IncludedObjects::read(b"not a chunk file").is_err());
}

#[test]
fn a_quaternion_turns_and_moves() -> TestResult {
    let half = std::f64::consts::FRAC_1_SQRT_2;

    let transform = Transform::from_quaternion([half, 0.0, 0.0, half], [1.0, 2.0, 3.0])
        .ok_or("no transform")?;

    assert!(
        close(transform.rows[0], [0.0, -1.0, 0.0, 1.0]),
        "{transform:?}"
    );
    assert!(
        close(transform.rows[1], [1.0, 0.0, 0.0, 2.0]),
        "{transform:?}"
    );
    assert!(
        close(transform.rows[2], [0.0, 0.0, 1.0, 3.0]),
        "{transform:?}"
    );
    assert!(Transform::from_quaternion([0.0; 4], [0.0; 3]).is_none());
    assert!(Transform::from_quaternion([1.0, 0.0, 0.0, 0.0], [f64::NAN, 0.0, 0.0]).is_none());
    Ok(())
}

#[test]
fn a_short_trailing_record_is_counted_not_an_error() -> TestResult {
    let mut chunk = TestSoc::new()
        .model("a.cgf")
        .object(0, SOC_IDENTITY)
        .chunk();
    let at = chunk.len() - 208 - 4;
    chunk[at..at + 4].copy_from_slice(&218_u32.to_le_bytes());
    chunk.extend_from_slice(&[0; 10]);
    let bytes = TestChunkFile::crch().chunk(0x0010, 1, chunk).bytes();

    let objects = IncludedObjects::read(&bytes)?.ok_or("no chunk")?;

    assert_eq!((objects.objects.len(), objects.invalid), (1, 1));
    Ok(())
}
