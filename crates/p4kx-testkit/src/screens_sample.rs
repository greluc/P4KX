// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::screens_canvas::{
    TestCanvas, TestCondition, TestModifier, TestStyle, TestStyleEntry, movie_component,
    screen_component,
};
use crate::screens_widget::{TestColour, TestDim, TestWidget};
use crate::{
    DataForgeBuilder, TestEntity, TestLoadout, TestLoadoutEntry, TestValue, entity_schema,
    screens_schema,
};

/// The GUID of the sample ship [`screens_sample`] holds.
pub const SCREEN_SHIP: u128 = 0x7d3a_1c90_58e2_4b6f_a1d4_3c2e_9b07_f811;

/// The GUID of the sample MFD canvas, 200 x 100 pixels with nested canvases.
pub const SCREEN_MFD: u128 = 0x2b8e_44a7_0d31_4c59_8f62_a9e1_70c3_5d24;

/// The GUID of the small panel canvas the MFD canvas nests.
pub const SCREEN_PANEL: u128 = 0x91c6_0f3d_a2b4_47e8_b5d9_16f8_c4a0_3e72;

/// The GUID of the HUD style of the manufacturer `test`.
pub const SCREEN_STYLE: u128 = 0xc5a2_81e9_37d0_4a6b_8e14_b7f3_90d6_2c58;

const SCREEN_TAG: u128 = 0x0f6d_2a83_c714_4e95_a3b8_5d90_e1c7_46f2;
const SCREEN_ENTITY_MFD: u128 = 0xa83c_5e17_92f4_4d06_b1a7_c8e2_3f95_d041;
const SCREEN_ENTITY_PANEL: u128 = 0x6b19_d4f0_e3a8_4c72_9d25_f1b6_08c7_a3e9;

const fn colour(red: u8, green: u8, blue: u8) -> TestColour {
    TestColour::Solid([red, green, blue, 255])
}

fn palette() -> TestStyle {
    TestStyle::new(
        SCREEN_STYLE,
        "s_test_hud",
        vec![
            colour(20, 160, 200),
            colour(255, 170, 0),
            colour(240, 240, 240),
        ],
    )
}

fn mfd() -> TestCanvas {
    let card = TestStyleEntry::new("cards")
        .when(TestCondition::Type("Card"))
        .set(TestModifier::EnableBackground(true))
        .set(TestModifier::Background(TestColour::Slot("Base", 0.5)));
    let tagged = TestStyleEntry::new("tagged")
        .when(TestCondition::Type("Base"))
        .when(TestCondition::Tag(SCREEN_TAG))
        .set(TestModifier::EnableBackground(true))
        .set(TestModifier::Background(TestColour::Slot("Bright", 1.0)));
    column_widgets()
        .into_iter()
        .chain(row_widgets())
        .chain(loose_widgets())
        .fold(
            TestCanvas::new(SCREEN_MFD, "test_mfd", 200.0, 100.0)
                .brand_style(SCREEN_STYLE, vec![card, tagged]),
            TestCanvas::widget,
        )
}

fn column_widgets() -> Vec<TestWidget> {
    vec![
        TestWidget::display("root")
            .background(colour(10, 16, 24))
            .flex("Column", "Start", "Start", 4.0),
        TestWidget::new("BuildingBlocks_WidgetCard", "header")
            .parent("root")
            .size(TestDim::Percent(1.0), TestDim::Fixed(20.0)),
        TestWidget::display("row")
            .parent("root")
            .size(TestDim::Percent(1.0), TestDim::Fixed(40.0))
            .padding([4.0, 4.0, 4.0, 4.0])
            .flex("Row", "Start", "Start", 6.0),
    ]
}

fn row_widgets() -> Vec<TestWidget> {
    vec![
        TestWidget::new("BuildingBlocks_WidgetCircle", "dot")
            .parent("row")
            .size(TestDim::Fixed(32.0), TestDim::Fixed(32.0))
            .with("doFill", TestValue::Bool(true))
            .with("fillColor", TestColour::Slot("Accent1", 1.0).value()),
        TestWidget::new("BuildingBlocks_WidgetPolygon", "arrow")
            .parent("row")
            .size(TestDim::OfHeight(1.0), TestDim::Fixed(32.0))
            .with("doFill", TestValue::Bool(true))
            .with("sides", TestValue::Int32(3))
            .with("startAngle", TestValue::Single(270.0))
            .with("fillColor", colour(255, 255, 255).value()),
        TestWidget::display("badge")
            .parent("row")
            .size(TestDim::Fixed(8.0), TestDim::Fixed(8.0))
            .tag(SCREEN_TAG),
        TestWidget::new("BuildingBlocks_WidgetText", "label")
            .parent("row")
            .size(TestDim::Fixed(30.0), TestDim::Fixed(10.0))
            .with("locString", TestValue::text("@test_label")),
        TestWidget::new("BuildingBlocks_WidgetCanvas", "panel")
            .parent("row")
            .size(TestDim::Fixed(0.0), TestDim::Percent(1.0))
            .grow(1.0)
            .with("canvas", TestValue::Reference(SCREEN_PANEL)),
    ]
}

fn loose_widgets() -> Vec<TestWidget> {
    vec![
        TestWidget::new("BuildingBlocks_WidgetImage", "picture")
            .parent("root")
            .size(TestDim::Fixed(10.0), TestDim::Fixed(10.0))
            .out_of_flow()
            .with("imagePath", TestValue::text("UI/Textures/Test/picture.tif")),
        TestWidget::display("hidden")
            .parent("root")
            .background(colour(255, 0, 255))
            .out_of_flow()
            .hidden(),
    ]
}

fn panel() -> TestCanvas {
    TestCanvas::new(SCREEN_PANEL, "test_panel", 100.0, 50.0)
        .widget(TestWidget::display("frame").border(2.0, colour(200, 30, 30)))
        .widget(
            TestWidget::display("fill")
                .parent("frame")
                .size(TestDim::Percent(0.5), TestDim::Percent(0.5))
                .offset(10.0, 10.0)
                .background(TestColour::Solid([30, 200, 60, 200])),
        )
        .widget(
            TestWidget::new("BuildingBlocks_WidgetCustomShape", "glyph")
                .parent("frame")
                .with(
                    "svgPath",
                    TestValue::text("UI/Textures/Vector/Test/glyph.svg"),
                ),
        )
}

/// Returns a synthetic DataForge with a ship whose default Loadout holds an MFD screen and a
/// panel screen, and the `BuildingBlocks` Records behind them.
///
/// The MFD canvas ([`SCREEN_MFD`], 200 x 100) lays out a column of a card whose background a
/// style entry sets, and a row with a circle, a triangle, a tagged badge a style entry fills,
/// a text no renderer draws, and the nested panel canvas ([`SCREEN_PANEL`]); it also holds a
/// picture and a hidden widget. The palette is the HUD style [`SCREEN_STYLE`] of the
/// manufacturer `test`. The ship has no UI movie; the MFD Entity names `Data/UI/test.gfx`.
#[must_use]
pub fn screens_sample() -> DataForgeBuilder {
    let ship = TestEntity::new(
        SCREEN_SHIP,
        "TEST_ScreenShip",
        "libs/foundry/records/entities/spaceships/test_screen_ship.xml",
    )
    .loadout(TestLoadout::Manual(vec![
        TestLoadoutEntry::class("Left_MFD", "TEST_Screen_MFD"),
        TestLoadoutEntry::class("Radar", "TEST_Screen_Panel"),
    ]));
    let mfd_screen = TestEntity::new(
        SCREEN_ENTITY_MFD,
        "TEST_Screen_MFD",
        "libs/foundry/records/entities/scitem/ships/test_screen_mfd.xml",
    )
    .component(screen_component(SCREEN_MFD))
    .component(movie_component("Data/UI/test.gfx"));
    let panel_screen = TestEntity::new(
        SCREEN_ENTITY_PANEL,
        "TEST_Screen_Panel",
        "libs/foundry/records/entities/scitem/ships/test_screen_panel.xml",
    )
    .component(screen_component(SCREEN_PANEL));
    [ship, mfd_screen, panel_screen]
        .iter()
        .map(TestEntity::record)
        .fold(
            screens_schema(entity_schema(DataForgeBuilder::new())),
            DataForgeBuilder::record,
        )
        .record(palette().record())
        .record(crate::tag_record(SCREEN_TAG, "Badge"))
        .record(mfd().record())
        .record(panel().record())
}
