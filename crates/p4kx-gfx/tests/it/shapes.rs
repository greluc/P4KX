//! Shapes and morph shapes: bounds, fill and line styles, edge records and the JSON form.

use p4kx_gfx::{FillStyle, Movie, Point, Rect, Rgba, ShapeRecord};
use p4kx_testkit::{
    TestFill, TestMovie, TestShape, TestShapeRecord, TestSignature, morph_shape_body,
};

const RED: [u8; 4] = [255, 0, 0, 128];
const BLUE: [u8; 4] = [0, 0, 255, 255];

fn square(id: u16, version: u8) -> TestShape {
    TestShape::new(id, version, [0, 2000, 0, 2000])
        .fill(TestFill::Solid(RED))
        .fill(TestFill::Linear(RED, BLUE))
        .fill(TestFill::Bitmap(9, 100, -40))
        .line(20, BLUE)
        .record(TestShapeRecord::Move {
            to: (0, 0),
            fill0: 1,
            fill1: 0,
            line: 1,
        })
        .record(TestShapeRecord::Line(2000, 0))
        .record(TestShapeRecord::Line(0, 2000))
        .record(TestShapeRecord::Line(-1000, -1000))
        .record(TestShapeRecord::Curve(-500, -500, -500, -500))
}

fn shapes_of(shape: &TestShape) -> Movie {
    Movie::parse(
        TestMovie::new(TestSignature::Gfx)
            .tag(shape.code(), &shape.body())
            .bytes(),
    )
    .unwrap_or_else(|error| unreachable!("invalid movie: {error}"))
}

#[test]
fn every_define_shape_version_reads_bounds_styles_and_records() {
    for version in 1..=4 {
        let movie = shapes_of(&square(7, version));
        let assets = movie.assets();
        assert!(assets.failures.is_empty(), "{:?}", assets.failures);
        let shape = &assets.shapes[0];
        assert_eq!(shape.id, 7, "v{version}");
        assert_eq!(shape.version, version, "v{version}");
        let bounds = Rect {
            x_min: 0,
            x_max: 2000,
            y_min: 0,
            y_max: 2000,
        };
        assert_eq!(shape.bounds, bounds, "v{version}");
        assert_eq!(
            shape.edge_bounds,
            (version == 4).then_some(bounds),
            "v{version}"
        );
        let alpha = if version >= 3 { 128 } else { 255 };
        assert_eq!(
            shape.fill_styles[0],
            FillStyle::Solid {
                color: Rgba {
                    red: 255,
                    green: 0,
                    blue: 0,
                    alpha
                }
            },
            "v{version}"
        );
        assert!(
            matches!(&shape.fill_styles[1], FillStyle::LinearGradient { gradient, .. } if gradient.stops.len() == 2 && gradient.stops[1].ratio == 255),
            "v{version}"
        );
        assert!(
            matches!(&shape.fill_styles[2], FillStyle::Bitmap { bitmap_id: 9, matrix, repeating: true, smoothed: true } if matrix.translate_x == 100 && matrix.translate_y == -40),
            "v{version}"
        );
        assert_eq!(shape.line_styles[0].width, 20, "v{version}");
        assert_eq!(shape.line_styles[0].color.blue, 255, "v{version}");
        assert_records(&shape.records, version);
    }
}

fn assert_records(records: &[ShapeRecord], version: u8) {
    let [
        ShapeRecord::StyleChange(change),
        horizontal,
        vertical,
        general,
        curve,
    ] = records
    else {
        unreachable!("v{version}: a style change and four edges, not {records:?}");
    };
    assert_eq!(change.move_to, Some(Point { x: 0, y: 0 }), "v{version}");
    assert_eq!(change.fill_style0, Some(1), "v{version}");
    assert_eq!(change.fill_style1, Some(0), "v{version}");
    assert_eq!(change.line_style, Some(1), "v{version}");
    let edge = |x, y| ShapeRecord::StraightEdge {
        delta: Point { x, y },
    };
    assert_eq!(*horizontal, edge(2000, 0), "horizontal, v{version}");
    assert_eq!(*vertical, edge(0, 2000), "vertical, v{version}");
    assert_eq!(*general, edge(-1000, -1000), "general, v{version}");
    assert_eq!(
        *curve,
        ShapeRecord::CurvedEdge {
            control: Point { x: -500, y: -500 },
            anchor: Point { x: -500, y: -500 },
        },
        "curve, v{version}"
    );
}

#[test]
fn a_shape_serialises_to_json_in_twips() {
    let movie = shapes_of(&square(3, 3));
    let assets = movie.assets();
    let json = serde_json::to_value(&assets.shapes[0]).expect("serialises");
    assert_eq!(json["id"], 3, "id");
    assert_eq!(json["bounds"]["x_max"], 2000, "bounds in twips");
    assert_eq!(json["fill_styles"][0]["type"], "solid", "tagged fills");
    assert_eq!(json["records"][0]["type"], "style_change", "tagged records");
    assert_eq!(json["records"][1]["delta"]["x"], 2000, "edge delta");
}

#[test]
fn a_morph_shape_reads_both_states() {
    let start = [
        TestShapeRecord::Move {
            to: (10, 10),
            fill0: 1,
            fill1: 0,
            line: 0,
        },
        TestShapeRecord::Line(100, 0),
    ];
    let end = [
        TestShapeRecord::Move {
            to: (20, 20),
            fill0: 0,
            fill1: 0,
            line: 0,
        },
        TestShapeRecord::Line(300, 0),
    ];
    let body = morph_shape_body(12, (RED, BLUE), &start, &end);
    let movie = Movie::parse(TestMovie::new(TestSignature::Fws).tag(46, &body).bytes())
        .expect("valid movie");
    let assets = movie.assets();
    assert!(assets.failures.is_empty(), "{:?}", assets.failures);
    let morph = &assets.morph_shapes[0];
    assert_eq!(morph.id, 12, "id");
    assert_eq!(morph.end_bounds.x_max, 400, "end bounds");
    assert!(
        matches!(morph.end_fill_styles[0], FillStyle::Solid { color } if color.blue == 255),
        "end colour"
    );
    assert_eq!(morph.start_records.len(), 2, "start records");
    assert_eq!(
        morph.end_records[1],
        ShapeRecord::StraightEdge {
            delta: Point { x: 300, y: 0 }
        },
        "end edge"
    );
}

#[test]
fn a_morph_shape_whose_end_edges_lie_beyond_its_tag_is_a_failure() {
    let mut body = morph_shape_body(12, (RED, BLUE), &[], &[]);
    let at = 2 + 6 + 6;
    body[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    let movie = Movie::parse(TestMovie::new(TestSignature::Fws).tag(46, &body).bytes())
        .expect("valid movie");
    let assets = movie.assets();
    assert!(assets.morph_shapes.is_empty(), "left out");
    assert_eq!(assets.failures.len(), 1, "reported");
}

#[test]
fn an_unknown_fill_style_is_a_failure() {
    let mut body = TestShape::new(1, 3, [0, 10, 0, 10])
        .fill(TestFill::Solid(RED))
        .body();
    body[2 + 4 + 1] = 0x77;
    let movie = Movie::parse(TestMovie::new(TestSignature::Gfx).tag(32, &body).bytes())
        .expect("valid movie");
    let assets = movie.assets();
    assert!(
        matches!(
            assets.failures[0].error,
            p4kx_gfx::GfxError::FillStyle { kind: 0x77 }
        ),
        "{:?}",
        assets.failures
    );
}
