// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::{Guid, StructValue};

use crate::colour::ColorSpec;
use crate::reader::{child, flag, float, reference, text};

pub(crate) const TEXT: &str = "BuildingBlocks_WidgetText";
pub(crate) const TEXT_FIELD: &str = "BuildingBlocks_WidgetTextField";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Horizontal {
    #[default]
    Left,
    Center,
    Right,
}

impl Horizontal {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "Left" => Some(Self::Left),
            "Center" => Some(Self::Center),
            "Right" => Some(Self::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Vertical {
    #[default]
    Top,
    Center,
    Bottom,
}

impl Vertical {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "Top" => Some(Self::Top),
            "Center" => Some(Self::Center),
            "Bottom" => Some(Self::Bottom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Case {
    #[default]
    Keep,
    Upper,
    Lower,
}

impl Case {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "None" => Some(Self::Keep),
            "Upper" => Some(Self::Upper),
            "Lower" => Some(Self::Lower),
            _ => None,
        }
    }

    pub(crate) fn apply(self, text: &str) -> String {
        match self {
            Self::Keep => text.to_owned(),
            Self::Upper => text.to_uppercase(),
            Self::Lower => text.to_lowercase(),
        }
    }
}

fn finite(structure: &StructValue<'_>, name: &str) -> Option<f32> {
    float(structure, name).filter(|value| value.is_finite())
}

#[derive(Debug, Clone)]
pub(crate) struct TextSpec {
    pub(crate) source: String,
    pub(crate) font_style: Option<Guid>,
    pub(crate) font_size: Option<f32>,
    pub(crate) auto_size: bool,
    pub(crate) colour: Option<ColorSpec>,
    pub(crate) letter_spacing: f32,
    pub(crate) line_spacing: f32,
    pub(crate) horizontal: Horizontal,
    pub(crate) vertical: Vertical,
    pub(crate) wrap: bool,
    pub(crate) case: Case,
    pub(crate) kerning: bool,
    pub(crate) preset: Option<String>,
}

impl TextSpec {
    pub(crate) fn parse(structure: &StructValue<'_>) -> Option<Self> {
        match structure.type_name() {
            TEXT => Some(Self::of_text(structure)),
            TEXT_FIELD => Some(Self::of_field(structure)),
            _ => None,
        }
    }

    fn of_text(structure: &StructValue<'_>) -> Self {
        Self {
            source: text(structure, "locString").unwrap_or_default().to_owned(),
            font_style: reference(structure, "fontStyle").map(p4kx_dataforge::Reference::guid),
            font_size: finite(structure, "fontSize"),
            auto_size: flag(structure, "autoFontSize").unwrap_or(false),
            colour: structure.field("color").and_then(ColorSpec::parse),
            letter_spacing: finite(structure, "letterSpacing").unwrap_or(0.0),
            line_spacing: finite(structure, "lineSpacing").unwrap_or(0.0),
            horizontal: text(structure, "textAlignment")
                .and_then(Horizontal::parse)
                .unwrap_or_default(),
            vertical: text(structure, "verticalAlignment")
                .and_then(Vertical::parse)
                .unwrap_or_default(),
            wrap: flag(structure, "wordWrap").unwrap_or(false),
            case: text(structure, "caseModifier")
                .and_then(Case::parse)
                .unwrap_or_default(),
            kerning: flag(structure, "kerning").unwrap_or(false),
            preset: None,
        }
    }

    fn of_field(structure: &StructValue<'_>) -> Self {
        let label = child(structure, "labelProperties");
        Self {
            source: label
                .and_then(|label| text(label, "label"))
                .unwrap_or_default()
                .to_owned(),
            font_style: None,
            font_size: None,
            auto_size: false,
            colour: None,
            letter_spacing: 0.0,
            line_spacing: 0.0,
            horizontal: text(structure, "textAlignment")
                .and_then(Horizontal::parse)
                .unwrap_or_default(),
            vertical: text(structure, "verticalTextAlignment")
                .and_then(Vertical::parse)
                .unwrap_or_default(),
            wrap: false,
            case: label
                .and_then(|label| text(label, "caseModifier"))
                .and_then(Case::parse)
                .unwrap_or_default(),
            kerning: false,
            preset: label
                .and_then(|label| text(label, "style"))
                .map(str::to_owned),
        }
    }
}
