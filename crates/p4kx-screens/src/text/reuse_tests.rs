// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;

use p4kx_dataforge::DataForge;
use p4kx_gfx::{Background, CanvasSize, Movie, RasterOptions};
use p4kx_testkit::{
    SCREEN_TEXT_STYLE, TestCanvas, TestColour, TestDim, TestOp, TestParam, TestValue, TestWidget,
    screens_font_movie, screens_text_sample,
};

use super::probe;
use crate::{ScreenComposer, TextSources};

type TestResult = Result<(), Box<dyn Error>>;

const OUTER: u128 = 0x5a00_0000_0000_4000_8000_0000_0000_0020;
const INNER: u128 = OUTER + 0x100;
const PATH: &str = "libs/foundry/records/ui/buildingblocks/test/reuse_outer";
const RED: TestColour = TestColour::Solid([255, 0, 0, 255]);
const GREEN: TestColour = TestColour::Solid([0, 255, 0, 255]);

struct Variant {
    default: &'static str,
    auto_size: bool,
    row_height: f32,
    padding: f32,
}

impl Variant {
    const fn fixed() -> Self {
        Self {
            default: "A",
            auto_size: false,
            row_height: 20.0,
            padding: 0.0,
        }
    }
}

fn label(variant: &Variant) -> TestWidget {
    TestWidget::new("BuildingBlocks_WidgetText", "label")
        .parent("row")
        .size(TestDim::Auto, TestDim::Fixed(20.0))
        .padding([variant.padding; 4])
        .with("locString", TestValue::text("A"))
        .with("fontStyle", TestValue::Reference(SCREEN_TEXT_STYLE))
        .with("fontSize", TestValue::Single(10.0))
        .with("autoFontSize", TestValue::Bool(variant.auto_size))
        .with("color", RED.value())
}

fn inner(variant: &Variant) -> TestCanvas {
    TestCanvas::new(INNER, "reuse_inner", 40.0, 30.0)
        .widget(
            TestWidget::display("row")
                .size(TestDim::Fixed(40.0), TestDim::Fixed(variant.row_height))
                .flex("Row", "Start", "Start", 0.0),
        )
        .widget(label(variant))
        .widget(
            TestWidget::display("bar")
                .parent("row")
                .size(TestDim::Fixed(6.0), TestDim::Fixed(20.0))
                .background(GREEN),
        )
        .operation("text", TestOp::TextParam("Label", variant.default))
        .operation("bind", TestOp::TextOf("label", "text"))
}

fn holder(name: &str, params: &[TestParam]) -> TestWidget {
    TestWidget::new("BuildingBlocks_WidgetCanvas", name)
        .parent("holders")
        .size(TestDim::Fixed(40.0), TestDim::Fixed(30.0))
        .with("canvas", TestValue::Reference(INNER))
        .params(params)
}

fn outer(holders: Vec<TestWidget>) -> TestCanvas {
    holders.into_iter().fold(
        TestCanvas::new(OUTER, "reuse_outer", 200.0, 100.0).widget(
            TestWidget::display("holders")
                .size(TestDim::Fixed(160.0), TestDim::Fixed(30.0))
                .flex("Row", "Start", "Start", 0.0),
        ),
        TestCanvas::widget,
    )
}

struct Outcome {
    width: u32,
    pixels: Vec<u8>,
    shaped: usize,
}

impl Outcome {
    fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let at = (y as usize * self.width as usize + x as usize) * 4;
        self.pixels
            .get(at..at + 4)
            .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            .unwrap_or_default()
    }

    fn is_green(&self, x: u32, y: u32) -> bool {
        self.at(x, y) == [0, 255, 0, 255]
    }
}

fn run(outer: &TestCanvas, inner: &TestCanvas, reuse: bool) -> Result<Outcome, Box<dyn Error>> {
    let bytes = screens_text_sample()
        .record(outer.record())
        .record(inner.record())
        .build();
    let forge = DataForge::open(&bytes)?;
    let fonts = Movie::parse(screens_font_movie())?.assets().fonts;
    probe::set_reuse(reuse);
    probe::take_shaped();
    let composed = ScreenComposer::new(&forge)
        .with_texts(TextSources {
            fonts: &fonts,
            localise: None,
        })
        .compose(PATH, None)?;
    let shaped = probe::take_shaped();
    probe::set_reuse(true);
    let raster = composed.rasterize(&RasterOptions {
        size: CanvasSize::Stage,
        background: Background::Transparent,
    })?;
    Ok(Outcome {
        width: raster.image.width(),
        pixels: raster.image.pixels().to_vec(),
        shaped,
    })
}

#[test]
fn two_instances_of_one_canvas_measure_the_text_each_hands_them() -> TestResult {
    let variant = Variant::fixed();
    let canvas = outer(vec![
        holder("long", &[TestParam::Text("Label", "AB")]),
        holder("short", &[TestParam::Text("Label", "A")]),
    ]);
    for reuse in [true, false] {
        let image = run(&canvas, &inner(&variant), reuse)?;
        assert!(image.is_green(14, 15), "bar after AB, reuse {reuse}");
        assert!(!image.is_green(8, 15), "no bar inside AB, reuse {reuse}");
        assert!(image.is_green(48, 15), "bar after A, reuse {reuse}");
        assert!(!image.is_green(44, 15), "no bar inside A, reuse {reuse}");
        assert!(
            !image.is_green(54, 15),
            "and not further out, reuse {reuse}"
        );
    }
    Ok(())
}

#[test]
fn the_text_measured_is_shaped_once_when_the_paint_pass_can_use_it() -> TestResult {
    let variant = Variant::fixed();
    let canvas = outer(vec![holder("only", &[])]);
    let kept = run(&canvas, &inner(&variant), true)?;
    let again = run(&canvas, &inner(&variant), false)?;
    assert_eq!(kept.shaped, 1, "shaped once");
    assert_eq!(again.shaped, 2, "shaped to measure and again to paint");
    assert_eq!(kept.pixels, again.pixels);
    Ok(())
}

#[test]
fn a_text_in_a_taller_padded_row_paints_as_it_does_without_reuse() -> TestResult {
    let variant = Variant {
        row_height: 30.0,
        padding: 5.0,
        ..Variant::fixed()
    };
    let canvas = outer(vec![holder("only", &[])]);
    let kept = run(&canvas, &inner(&variant), true)?;
    let again = run(&canvas, &inner(&variant), false)?;
    assert_eq!(kept.shaped, 1);
    assert_eq!(kept.pixels, again.pixels);
    Ok(())
}

#[test]
fn a_text_that_fits_its_width_paints_as_it_does_without_reuse() -> TestResult {
    let canvas = outer(vec![holder("only", &[])]);
    for default in ["A", "ABABABABAB"] {
        let variant = Variant {
            default,
            auto_size: true,
            ..Variant::fixed()
        };
        let kept = run(&canvas, &inner(&variant), true)?;
        let again = run(&canvas, &inner(&variant), false)?;
        assert_eq!(kept.pixels, again.pixels, "{default}");
        assert!(kept.shaped <= again.shaped, "{default}");
    }
    Ok(())
}
