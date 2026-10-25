// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::screens_canvas::record_in;
use crate::screens_widget::TestColour;
use crate::{TestInstance, TestRecord, TestValue};

const STYLE_FOLDER: &str = "libs/foundry/records/ui/buildingblocks/styles/";

/// A selector of a synthetic style entry.
#[derive(Debug, Clone, PartialEq)]
pub enum TestCondition {
    /// The widget type, as the style data names it: `Card`, `Base` for a display widget, ...
    Type(&'static str),
    /// The widget has the `Tag` Record `guid` or one below it.
    Tag(u128),
    /// The parent widget meets every condition.
    Parent(Vec<Self>),
    /// Some ancestor widget meets every condition.
    Ancestor(Vec<Self>),
    /// A selector the screens reader does not evaluate.
    Count,
}

impl TestCondition {
    fn value(&self) -> TestValue {
        TestValue::Struct(match self {
            Self::Type(kind) => TestInstance::new("BuildingBlocks_StyleSelectorConditionType")
                .value("type", TestValue::text(*kind)),
            Self::Tag(guid) => TestInstance::new("BuildingBlocks_StyleSelectorConditionTag")
                .value("tag", TestValue::Reference(*guid)),
            Self::Parent(inner) => TestInstance::new("BuildingBlocks_StyleSelectorConditionParent")
                .value(
                    "conditions",
                    TestValue::Array(inner.iter().map(Self::value).collect()),
                ),
            Self::Ancestor(inner) => {
                TestInstance::new("BuildingBlocks_StyleSelectorConditionAncestor").value(
                    "conditions",
                    TestValue::Array(inner.iter().map(Self::value).collect()),
                )
            }
            Self::Count => TestInstance::new("BuildingBlocks_StyleSelectorConditionCount"),
        })
    }
}

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
    /// Any numeric field by its name, e.g. `AnchorX`, `SizeX` or `OrientationZ`.
    Number(&'static str, f32),
}

impl TestModifier {
    pub(crate) fn value(self) -> TestValue {
        let (kind, field, value) = match self {
            Self::Number(field, value) => ("Number", field, TestValue::Single(value)),
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

/// One style entry: when its conditions hold, its modifiers apply.
#[derive(Debug, Clone, PartialEq)]
pub struct TestStyleEntry {
    name: String,
    conditions: Vec<TestCondition>,
    modifiers: Vec<TestModifier>,
}

impl TestStyleEntry {
    /// Creates an entry named `name` without conditions or modifiers.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            conditions: Vec::new(),
            modifiers: Vec::new(),
        }
    }

    /// Adds a condition that must hold together with the others.
    #[must_use]
    pub fn when(mut self, condition: TestCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Adds a modifier.
    #[must_use]
    pub fn set(mut self, modifier: TestModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    fn value(&self) -> TestValue {
        TestValue::Struct(
            TestInstance::new("BuildingBlocks_StyleEntry")
                .value("name", TestValue::text(self.name.clone()))
                .value(
                    "conditionsList",
                    TestValue::Array(vec![TestValue::Struct(
                        TestInstance::new("BuildingBlocks_StyleConditionList")
                            .value("name", TestValue::text("list"))
                            .value(
                                "conditions",
                                TestValue::Array(
                                    self.conditions.iter().map(TestCondition::value).collect(),
                                ),
                            ),
                    )]),
                )
                .value(
                    "modifiers",
                    TestValue::Array(self.modifiers.iter().map(|m| m.value()).collect()),
                ),
        )
    }
}

pub(crate) fn entries(entries: &[TestStyleEntry]) -> TestValue {
    TestValue::Array(entries.iter().map(TestStyleEntry::value).collect())
}

/// A synthetic `BuildingBlocks_Style` Record: a palette and style entries.
#[derive(Debug, Clone, PartialEq)]
pub struct TestStyle {
    guid: u128,
    name: String,
    palette: Vec<TestColour>,
    entries: Vec<TestStyleEntry>,
}

impl TestStyle {
    /// Creates a style whose palette lists `palette` for the slots of
    /// [`SCREEN_COLOUR_SLOTS`](crate::SCREEN_COLOUR_SLOTS) in order.
    #[must_use]
    pub fn new(guid: u128, name: impl Into<String>, palette: Vec<TestColour>) -> Self {
        Self {
            guid,
            name: name.into(),
            palette,
            entries: Vec::new(),
        }
    }

    /// Adds a style entry.
    #[must_use]
    pub fn entry(mut self, entry: TestStyleEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Returns the Record, at the record path `.../styles/<name>` where the HUD styles of the
    /// manufacturers are.
    #[must_use]
    pub fn record(&self) -> TestRecord {
        record_in(STYLE_FOLDER, self.guid, "BuildingBlocks_Style", &self.name)
            .value("entries", entries(&self.entries))
            .value(
                "colorStyles",
                TestValue::Array(self.palette.iter().map(|c| c.value()).collect()),
            )
    }
}
