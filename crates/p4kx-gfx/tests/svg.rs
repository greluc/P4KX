//! SVG output: the structure of the document for shapes, gradients, strokes, transforms, colour
//! transforms, its determinism, and the escaping of text.

mod svg_support;

use p4kx_gfx::{Background, CanvasSize, GfxError, RasterOptions};
use p4kx_testkit::{TestFill, TestGradient, TestPlace, TestShape, TestShapeRecord, sprite_body};
use roxmltree::Node;
use svg_support::{BLUE, RED, XLINK, elements, options, parse, rect, stage, svg, write_with};

fn square_at(x: i32, y: i32, fill: TestFill) -> Vec<u8> {
    let square = rect(4, (1000, 1000), fill);
    stage()
        .tag(square.code(), &square.body())
        .tag(26, &TestPlace::new(1, 4).at(x, y).body2())
        .tag(1, &[])
        .bytes()
}

#[test]
fn a_solid_shape_is_one_group_placed_with_its_transform() {
    let text = svg(&square_at(200, 300, TestFill::Solid(RED)));
    let document = parse(&text);
    let root = document.root_element();
    assert_eq!(root.tag_name().name(), "svg");
    assert_eq!(root.attribute("width"), Some("100"));
    assert_eq!(root.attribute("height"), Some("100"));
    assert_eq!(root.attribute("viewBox"), Some("0 0 100 100"));
    let paths = elements(&document, "path");
    assert_eq!(paths.len(), 1, "one path");
    assert_eq!(paths[0].attribute("fill"), Some("#ff0000"));
    assert_eq!(paths[0].attribute("fill-rule"), Some("evenodd"));
    assert_eq!(paths[0].attribute("fill-opacity"), None, "opaque");
    assert!(
        paths[0]
            .attribute("d")
            .is_some_and(|d| d.starts_with("M0 0 ") && d.ends_with('Z')),
        "{:?}",
        paths[0].attribute("d")
    );
    let uses = elements(&document, "use");
    assert_eq!(uses.len(), 1);
    assert_eq!(uses[0].attribute((XLINK, "href")), Some("#s0"));
    assert_eq!(
        uses[0].attribute("transform"),
        Some("matrix(1 0 0 1 200 300)")
    );
    let groups = elements(&document, "g");
    assert_eq!(
        groups[0].attribute("id"),
        Some("s0"),
        "the shape, in the definitions"
    );
    assert_eq!(
        groups[1].attribute("transform"),
        Some("matrix(0.05 0 0 0.05 0 0)"),
        "twips to pixels"
    );
}

#[test]
fn the_output_is_deterministic_and_follows_the_canvas_size() {
    let bytes = square_at(0, 0, TestFill::Solid(RED));
    assert_eq!(svg(&bytes), svg(&bytes), "identical bytes");
    let wide = write_with(&bytes, &options(CanvasSize::Width(250)), None, true)
        .unwrap_or_else(|error| unreachable!("{error}"));
    assert_eq!((wide.1.width, wide.1.height), (250, 250));
    assert!(wide.0.contains("viewBox=\"0 0 250 250\""), "{}", wide.0);
    assert!(wide.0.contains("matrix(0.125 0 0 0.125 0 0)"), "{}", wide.0);
    let fit = write_with(
        &bytes,
        &options(CanvasSize::Fit {
            width: 200,
            height: 100,
        }),
        None,
        true,
    )
    .unwrap_or_else(|error| unreachable!("{error}"));
    assert!(
        fit.0.contains("matrix(0.05 0 0 0.05 50 0)"),
        "centred: {}",
        fit.0
    );
}

#[test]
fn a_background_colour_is_a_rectangle() {
    let bytes = square_at(0, 0, TestFill::Solid(RED));
    let options = RasterOptions {
        size: CanvasSize::Stage,
        background: Background::Color(p4kx_gfx::Rgba {
            red: 0,
            green: 0,
            blue: 255,
            alpha: 128,
        }),
    };
    let written =
        write_with(&bytes, &options, None, true).unwrap_or_else(|error| unreachable!("{error}"));
    let document = parse(&written.0);
    let rects = elements(&document, "rect");
    assert_eq!(rects.len(), 1);
    assert_eq!(rects[0].attribute("fill"), Some("#0000ff"));
    assert_eq!(rects[0].attribute("fill-opacity"), Some("0.502"));
}

#[test]
fn a_shape_placed_twice_is_written_once_and_a_colour_transform_makes_a_second_group() {
    let square = rect(4, (1000, 1000), TestFill::Solid(RED));
    let bytes = stage()
        .tag(square.code(), &square.body())
        .tag(26, &TestPlace::new(1, 4).body2())
        .tag(26, &TestPlace::new(2, 4).at(1200, 0).body2())
        .tag(
            26,
            &TestPlace::new(3, 4)
                .at(0, 1200)
                .color([0.0, 1.0, 1.0, 0.5], [0, 255, 0, 0])
                .body2(),
        )
        .tag(1, &[])
        .bytes();
    let text = svg(&bytes);
    let document = parse(&text);
    let paths = elements(&document, "path");
    assert_eq!(paths.len(), 2, "plain and tinted");
    assert_eq!(paths[0].attribute("fill"), Some("#ff0000"));
    assert_eq!(paths[1].attribute("fill"), Some("#00ff00"));
    assert_eq!(paths[1].attribute("fill-opacity"), Some("0.502"));
    let hrefs: Vec<_> = elements(&document, "use")
        .iter()
        .map(|node| node.attribute((XLINK, "href")))
        .collect();
    assert_eq!(hrefs, [Some("#s0"), Some("#s0"), Some("#s1")]);
}

#[test]
fn sprites_place_their_contents_with_composed_transforms() {
    let square = rect(4, (1000, 1000), TestFill::Solid(RED));
    let sprite = sprite_body(
        9,
        1,
        &[
            (26, TestPlace::new(1, 4).at(200, 0).body2()),
            (1, Vec::new()),
        ],
    );
    let bytes = stage()
        .tag(square.code(), &square.body())
        .tag(39, &sprite)
        .tag(26, &TestPlace::new(1, 9).at(0, 200).body2())
        .tag(1, &[])
        .bytes();
    let text = svg(&bytes);
    let document = parse(&text);
    let uses = elements(&document, "use");
    assert_eq!(uses.len(), 1);
    assert_eq!(
        uses[0].attribute("transform"),
        Some("matrix(1 0 0 1 200 200)")
    );
}

#[test]
fn gradients_are_written_with_stops_transform_and_spread() {
    let linear = rect(
        1,
        (2000, 1000),
        TestFill::Gradient(RED, BLUE, TestGradient::Linear { x: 0, width: 2000 }),
    );
    let radial = rect(
        2,
        (2000, 1000),
        TestFill::Gradient(
            RED,
            BLUE,
            TestGradient::Radial {
                center: (1000, 500),
                radius: 500,
                focal: 192,
            },
        ),
    );
    let bytes = stage()
        .tag(linear.code(), &linear.body())
        .tag(radial.code(), &radial.body())
        .tag(26, &TestPlace::new(1, 1).body2())
        .tag(26, &TestPlace::new(2, 2).at(0, 1000).body2())
        .tag(1, &[])
        .bytes();
    let text = svg(&bytes);
    let document = parse(&text);
    let linear = elements(&document, "linearGradient");
    let radial = elements(&document, "radialGradient");
    assert_eq!((linear.len(), radial.len()), (1, 1));
    assert_eq!(linear[0].attribute("gradientUnits"), Some("userSpaceOnUse"));
    assert_eq!(linear[0].attribute("x1"), Some("-16384"));
    assert_eq!(linear[0].attribute("x2"), Some("16384"));
    assert!(
        linear[0]
            .attribute("gradientTransform")
            .is_some_and(|t| t.starts_with("matrix("))
    );
    assert_eq!(radial[0].attribute("r"), Some("16384"));
    assert_eq!(radial[0].attribute("fx"), Some("12288"));
    let stops: Vec<_> = linear[0]
        .children()
        .filter(Node::is_element)
        .map(|node| (node.attribute("offset"), node.attribute("stop-color")))
        .collect();
    assert_eq!(
        stops,
        [(Some("0"), Some("#ff0000")), (Some("1"), Some("#0000ff"))]
    );
    let paths = elements(&document, "path");
    let id = linear[0].attribute("id").unwrap_or_default();
    assert_eq!(
        paths[0].attribute("fill"),
        Some(format!("url(#{id})").as_str())
    );
}

#[test]
fn strokes_keep_their_width_caps_and_joins_and_hairlines_stay_one_pixel() {
    let line = |id: u16, width: u16| {
        TestShape::new(id, 3, [0, 2000, 0, 2000])
            .line(width, [0, 0, 0, 255])
            .record(TestShapeRecord::Move {
                to: (200, 1000),
                fill0: 0,
                fill1: 0,
                line: 1,
            })
            .record(TestShapeRecord::Line(1600, 0))
            .record(TestShapeRecord::Curve(0, 800, -800, 0))
    };
    let (thick, hair) = (line(1, 100), line(2, 0));
    let bytes = stage()
        .tag(thick.code(), &thick.body())
        .tag(hair.code(), &hair.body())
        .tag(26, &TestPlace::new(1, 1).body2())
        .tag(26, &TestPlace::new(2, 2).body2())
        .tag(1, &[])
        .bytes();
    let text = svg(&bytes);
    let document = parse(&text);
    let paths = elements(&document, "path");
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].attribute("fill"), Some("none"));
    assert_eq!(paths[0].attribute("stroke"), Some("#000000"));
    assert_eq!(paths[0].attribute("stroke-width"), Some("100"));
    assert!(paths[0].attribute("stroke-linecap").is_some());
    assert!(paths[0].attribute("stroke-linejoin").is_some());
    assert!(
        paths[0].attribute("d").is_some_and(|d| d.contains('Q')),
        "curve"
    );
    assert_eq!(paths[1].attribute("stroke-width"), Some("1"));
    assert_eq!(
        paths[1].attribute("vector-effect"),
        Some("non-scaling-stroke")
    );
}

#[test]
fn the_title_is_escaped_and_cannot_inject_markup() {
    let hostile = "</title><script>alert(1)</script>&\"'\u{1}";
    let (text, _) = write_with(
        &square_at(0, 0, TestFill::Solid(RED)),
        &options(CanvasSize::Stage),
        Some(hostile),
        true,
    )
    .unwrap_or_else(|error| unreachable!("{error}"));
    assert!(!text.contains("<script"), "{text}");
    let document = parse(&text);
    assert_eq!(elements(&document, "script").len(), 0);
    let title = &elements(&document, "title")[0];
    assert_eq!(
        title.text(),
        Some("</title><script>alert(1)</script>&\"'\u{FFFD}")
    );
}

#[test]
fn canvases_beyond_the_limits_are_errors() {
    let bytes = square_at(0, 0, TestFill::Solid(RED));
    for size in [
        CanvasSize::Width(16_384),
        CanvasSize::Fit {
            width: 0,
            height: 10,
        },
    ] {
        assert!(
            matches!(
                write_with(&bytes, &options(size), None, true),
                Err(GfxError::CanvasSize { .. })
            ),
            "{size:?}"
        );
    }
}
