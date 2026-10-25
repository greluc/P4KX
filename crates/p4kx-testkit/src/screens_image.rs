// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::screens_canvas::{
    TestCanvas, TestCondition, TestModifier, TestStyle, TestStyleEntry, tag_record,
};
use crate::screens_widget::{TestColour, TestDim, TestWidget};
use crate::{DataForgeBuilder, TestInstance, TestValue, entity_schema, screens_schema};

/// The GUID of the synthetic canvas [`screens_image_sample`] holds, 200 x 100 pixels.
pub const SCREEN_IMAGE_CANVAS: u128 = 0x7c15_e4a2_09b8_4d63_a1f7_2e9c_d580_36b4;

const SCREEN_IMAGE_STYLE: u128 = 0xe08b_6d21_4f37_4a95_b1c8_5a73_92f0_d6e4;
const SCREEN_IMAGE_TAG: u128 = 0x31a9_5f0c_d8e2_47b6_9a04_c6e1_72bd_f853;

const QUAD: [[u8; 4]; 4] = [
    [255, 0, 0, 255],
    [0, 255, 0, 255],
    [0, 0, 255, 255],
    [255, 255, 255, 255],
];

const BOX: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="#ffffff"/></svg>"##;

const BANDS: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 30 10">
<style type="text/css">.green { fill: #00ff00; }</style>
<g><rect width="10" height="10" fill="#ff0000"/><rect x="10" width="10" height="10" class="green"/><rect x="20" width="10" height="10" style="fill:rgb(0,0,255)"/></g>
</svg>"##;

const RING: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" fill="none" stroke="#ffffff" stroke-width="2"/><path d="M5 5 m-1 0 a1 1 0 1 0 2 0 a1 1 0 1 0 -2 0z" fill="#ff0000" transform="translate(0 0)"/></svg>"##;

const SLOT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect id="colorstyle:Accent1_opacity:50_0001" width="10" height="10" fill="#ffffff"/></svg>"##;

const REUSE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 10 10"><defs><rect id="dot" width="4" height="4" fill="#00ff00"/></defs><use xlink:href="#dot" x="3" y="3"/></svg>"##;

/// Returns the pixels of a synthetic texture of the screens, by its path ignoring case: width,
/// height and four bytes R, G, B, A per pixel.
///
/// `UI/Test/quad.dds` is 2 x 2 pixels: red and green above, blue and white below.
#[must_use]
pub fn screens_texture(path: &str) -> Option<(u32, u32, Vec<u8>)> {
    path.eq_ignore_ascii_case("UI/Test/quad.dds")
        .then(|| (2, 2, QUAD.concat()))
}

/// Returns the bytes of a synthetic SVG file of the screens, by its path ignoring case.
///
/// `UI/Test/box.svg` is a white 10 x 10 square, `UI/Test/bands.svg` a red, a green and a blue
/// 10 x 10 square side by side (with a DOCTYPE and a style rule), `UI/Test/ring.svg` a white ring
/// with a red dot made of arcs, `UI/Test/slot.svg` a square that names the palette slot
/// `Accent1` at half opacity, and `UI/Test/reuse.svg` a green 4 x 4 square a `use` element
/// places at 3, 3.
#[must_use]
pub fn screens_vector(path: &str) -> Option<Vec<u8>> {
    let text = [
        ("UI/Test/box.svg", BOX),
        ("UI/Test/bands.svg", BANDS),
        ("UI/Test/ring.svg", RING),
        ("UI/Test/slot.svg", SLOT),
        ("UI/Test/reuse.svg", REUSE),
        (
            "UI/Textures/Vector/General/ModularKit/Widgets/IconWidget/test_box_x.svg",
            BOX,
        ),
    ]
    .into_iter()
    .find(|(name, _)| name.eq_ignore_ascii_case(path))?
    .1;
    Some(text.as_bytes().to_vec())
}

fn solid(red: u8, green: u8, blue: u8) -> TestValue {
    TestColour::Solid([red, green, blue, 255]).value()
}

fn picture(name: &'static str, at: (f32, f32), size: (f32, f32)) -> TestWidget {
    TestWidget::new("BuildingBlocks_WidgetImage", name)
        .parent("root")
        .size(TestDim::Fixed(size.0), TestDim::Fixed(size.1))
        .offset(at.0, at.1)
        .with("imagePath", TestValue::text("UI/Test/quad.tif"))
}

fn shape(name: &'static str, at: (f32, f32), size: (f32, f32), svg: &'static str) -> TestWidget {
    TestWidget::new("BuildingBlocks_WidgetCustomShape", name)
        .parent("root")
        .size(TestDim::Fixed(size.0), TestDim::Fixed(size.1))
        .offset(at.0, at.1)
        .with("svgPath", TestValue::text(svg))
        .with("renderShape", TestValue::Bool(true))
        .with("enableColorOverlay", TestValue::Bool(true))
}

fn slice(left: f32, right: f32, top: f32, bottom: f32) -> TestValue {
    TestValue::Struct(
        TestInstance::new("BuildingBlocks_TRBL")
            .value("top", TestValue::Single(top))
            .value("right", TestValue::Single(right))
            .value("bottom", TestValue::Single(bottom))
            .value("left", TestValue::Single(left)),
    )
}

fn svg_fill(path: &'static str, colour: Option<TestValue>) -> TestValue {
    let fill = TestInstance::new("BuildingBlocks_SvgFill")
        .value("svgPath", TestValue::text(path))
        .value("renderShape", TestValue::Bool(true))
        .value("enableColorOverlay", TestValue::Bool(true));
    TestValue::Struct(match colour {
        Some(colour) => fill.value("color", colour),
        None => fill,
    })
}

fn icon(
    name: &'static str,
    at: (f32, f32),
    preset: &'static str,
    custom: &'static str,
) -> TestWidget {
    TestWidget::new("BuildingBlocks_WidgetIcon", name)
        .parent("root")
        .size(TestDim::Fixed(20.0), TestDim::Fixed(20.0))
        .offset(at.0, at.1)
        .with(
            "iconProperties",
            TestValue::Struct(
                TestInstance::new("BuildingBlocks_ComponentIconProperties")
                    .value("show", TestValue::Bool(true))
                    .value("iconPreset", TestValue::text(preset))
                    .value("customIcon", TestValue::text(custom)),
            ),
        )
        .with("svgFill", svg_fill("", Some(solid(0, 255, 255))))
}

fn images() -> Vec<TestWidget> {
    vec![
        picture("fill", (0.0, 0.0), (40.0, 20.0)),
        picture("contain", (50.0, 0.0), (40.0, 20.0))
            .with("imageScalingMethod", TestValue::text("Contain"))
            .with("containPositionX", TestValue::Single(0.5)),
        picture("cover", (100.0, 0.0), (40.0, 20.0))
            .with("imageScalingMethod", TestValue::text("Cover")),
        picture("flip", (150.0, 0.0), (40.0, 20.0)).with("flipH", TestValue::Bool(true)),
        picture("tint", (0.0, 30.0), (40.0, 20.0)).with("color", solid(255, 0, 0)),
        picture("ghost", (50.0, 30.0), (40.0, 20.0)).alpha(0.5),
        picture("missing", (100.0, 30.0), (40.0, 20.0))
            .with("imagePath", TestValue::text("UI/Test/missing.tif")),
        icon("icon", (150.0, 30.0), "_None", "UI/Test/box.svg"),
        icon("preset", (175.0, 30.0), "TestBoxX", ""),
    ]
}

fn shapes() -> Vec<TestWidget> {
    vec![
        shape("box", (0.0, 60.0), (20.0, 20.0), "UI/Test/box.svg").with("color", solid(0, 0, 255)),
        shape("bands", (30.0, 60.0), (60.0, 10.0), "UI/Test/bands.svg")
            .with("scalingBehavior", TestValue::text("StretchXY"))
            .with("enableColorOverlay", TestValue::Bool(false)),
        shape("sliced", (100.0, 60.0), (90.0, 10.0), "UI/Test/bands.svg")
            .with("scalingBehavior", TestValue::text("StretchXY"))
            .with("enableColorOverlay", TestValue::Bool(false))
            .with("enableNineSliceRect", TestValue::Bool(true))
            .with("nineSliceRect", slice(1.0 / 3.0, 2.0 / 3.0, 0.0, 1.0))
            .with("nineSliceScale", TestValue::Single(1.0)),
        shape("ring", (0.0, 80.0), (20.0, 20.0), "UI/Test/ring.svg")
            .with("enableColorOverlay", TestValue::Bool(false)),
        TestWidget::new("BuildingBlocks_WidgetImage", "inside")
            .parent("root")
            .size(TestDim::Fixed(20.0), TestDim::Fixed(20.0))
            .offset(30.0, 80.0)
            .with("imagePath", TestValue::text(""))
            .with(
                "svgFill",
                svg_fill("UI/Test/box.svg", Some(solid(255, 255, 0))),
            ),
        shape("cascaded", (60.0, 80.0), (20.0, 20.0), "").tag(SCREEN_IMAGE_TAG),
        shape("slot", (150.0, 80.0), (20.0, 20.0), "UI/Test/slot.svg")
            .with("enableColorOverlay", TestValue::Bool(false)),
        shape("reuse", (175.0, 80.0), (20.0, 20.0), "UI/Test/reuse.svg")
            .with("enableColorOverlay", TestValue::Bool(false)),
        TestWidget::new("BuildingBlocks_WidgetForegroundOverlay", "later")
            .parent("root")
            .size(TestDim::Fixed(20.0), TestDim::Fixed(20.0))
            .offset(90.0, 80.0)
            .with("urlOptional", TestValue::text("")),
        TestWidget::new("BuildingBlocks_WidgetForegroundOverlay", "overlay")
            .parent("root")
            .size(TestDim::Fixed(20.0), TestDim::Fixed(20.0))
            .offset(120.0, 80.0)
            .with("urlOptional", TestValue::text("UI/Test/quad.tif")),
    ]
}

fn palette() -> TestStyle {
    let colour = |red, green, blue| TestColour::Solid([red, green, blue, 255]);
    TestStyle::new(
        SCREEN_IMAGE_STYLE,
        "s_test_hud",
        vec![
            colour(20, 160, 200),
            colour(255, 170, 0),
            colour(240, 240, 240),
        ],
    )
}

fn canvas() -> TestCanvas {
    let cascaded = TestStyleEntry::new("cascaded")
        .when(TestCondition::Tag(SCREEN_IMAGE_TAG))
        .set(TestModifier::SvgPath("UI/Test/box.svg"))
        .set(TestModifier::Fill(TestColour::Solid([0, 255, 0, 255])));
    std::iter::once(TestWidget::display("root"))
        .chain(images())
        .chain(shapes())
        .fold(
            TestCanvas::new(SCREEN_IMAGE_CANVAS, "image_sample", 200.0, 100.0)
                .brand_style(SCREEN_IMAGE_STYLE, Vec::new())
                .embedded(cascaded),
            TestCanvas::widget,
        )
}

/// Returns a synthetic DataForge with the canvas [`SCREEN_IMAGE_CANVAS`], 200 x 100 pixels.
///
/// Its first rows hold images of [`screens_texture`] scaled to fill, contain and cover, flipped,
/// tinted, translucent, missing, and an icon; the rest holds custom shapes of [`screens_vector`]
/// scaled, with a nine-slice, overlaid with a colour, as the vector of an image widget, named by
/// a style entry, coloured by a palette slot or built from a `use` element, and two foreground
/// overlays. The palette is the HUD style of the manufacturer `test`.
#[must_use]
pub fn screens_image_sample() -> DataForgeBuilder {
    screens_schema(entity_schema(DataForgeBuilder::new()))
        .record(palette().record())
        .record(tag_record(SCREEN_IMAGE_TAG, "Cascaded"))
        .record(canvas().record())
}
