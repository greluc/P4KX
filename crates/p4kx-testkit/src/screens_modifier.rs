// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::screens_widget::TestColour;
use crate::{TestInstance, TestValue};

/// A modifier of a synthetic style entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TestModifier {
    /// `BackgroundColor`.
    Background(TestColour),
    /// `FillColor`.
    Fill(TestColour),
    /// `StrokeColor`.
    Stroke(TestColour),
    /// `EnableBackground`.
    EnableBackground(bool),
    /// `IsActive`.
    Active(bool),
    /// `Alpha`.
    Alpha(f32),
    /// `FontSize`.
    FontSize(f32),
    /// `LetterSpacing`.
    LetterSpacing(f32),
    /// `LineSpacing`.
    LineSpacing(f32),
    /// `WordWrap`.
    WordWrap(bool),
    /// The font style Record `guid`.
    FontStyle(u128),
    /// The horizontal alignment, `Left`, `Center` or `Right`.
    Horizontal(&'static str),
    /// The vertical alignment, `Top`, `Center` or `Bottom`.
    Vertical(&'static str),
    /// The case modifier, `None`, `Upper` or `Lower`.
    Case(&'static str),
    /// The localised `Text` field, e.g. a key as `@key`.
    Text(&'static str),
    /// `ImagePath`.
    ImagePath(&'static str),
    /// `SvgPath`.
    SvgPath(&'static str),
    /// `EnableColorOverlay`.
    ColourOverlay(bool),
}

impl TestModifier {
    pub(crate) fn value(self) -> TestValue {
        let (kind, field, value) = match self {
            Self::Background(colour) => ("Color", "BackgroundColor", colour.value()),
            Self::Fill(colour) => ("Color", "FillColor", colour.value()),
            Self::Stroke(colour) => ("Color", "StrokeColor", colour.value()),
            Self::EnableBackground(on) => ("Boolean", "EnableBackground", TestValue::Bool(on)),
            Self::Active(on) => ("Boolean", "IsActive", TestValue::Bool(on)),
            Self::WordWrap(on) => ("Boolean", "WordWrap", TestValue::Bool(on)),
            Self::Alpha(alpha) => ("Number", "Alpha", TestValue::Single(alpha)),
            Self::FontSize(size) => ("Number", "FontSize", TestValue::Single(size)),
            Self::LetterSpacing(space) => ("Number", "LetterSpacing", TestValue::Single(space)),
            Self::LineSpacing(space) => ("Number", "LineSpacing", TestValue::Single(space)),
            Self::Text(text) => ("Localization", "Text", TestValue::text(text)),
            Self::ImagePath(path) => ("String", "ImagePath", TestValue::text(path)),
            Self::SvgPath(path) => ("String", "SvgPath", TestValue::text(path)),
            Self::ColourOverlay(on) => ("Boolean", "EnableColorOverlay", TestValue::Bool(on)),
            Self::FontStyle(guid) => {
                return typed(
                    "RecordRef",
                    "RecordRefTypeFontStyleRecord",
                    TestValue::Reference(guid),
                );
            }
            Self::Horizontal(name) => {
                return typed(
                    "Enumerated",
                    "EnumeratedTypeHorizontalAlignment",
                    TestValue::text(name),
                );
            }
            Self::Vertical(name) => {
                return typed(
                    "Enumerated",
                    "EnumeratedTypeVerticalAlignment",
                    TestValue::text(name),
                );
            }
            Self::Case(name) => {
                return typed(
                    "Enumerated",
                    "EnumeratedTypeCaseModifier",
                    TestValue::text(name),
                );
            }
        };
        let key = if kind == "Color" { "color" } else { "value" };
        TestValue::Struct(
            TestInstance::new(format!("BuildingBlocks_FieldModifier{kind}"))
                .value("field", TestValue::text(field))
                .value(key, value),
        )
    }
}

fn typed(kind: &str, inner: &str, value: TestValue) -> TestValue {
    TestValue::Struct(
        TestInstance::new(format!("BuildingBlocks_FieldModifier{kind}")).value(
            "field",
            TestValue::Struct(
                TestInstance::new(format!("BuildingBlocks_FieldModifier{inner}"))
                    .value("value", value),
            ),
        ),
    )
}
