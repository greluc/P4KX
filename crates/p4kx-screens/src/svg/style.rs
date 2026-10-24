// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gfx::{Cap, Join};

use super::number::leading_number;

const NAMED: [(&str, [u8; 3]); 17] = [
    ("black", [0, 0, 0]),
    ("white", [255, 255, 255]),
    ("red", [255, 0, 0]),
    ("lime", [0, 255, 0]),
    ("green", [0, 128, 0]),
    ("blue", [0, 0, 255]),
    ("yellow", [255, 255, 0]),
    ("cyan", [0, 255, 255]),
    ("aqua", [0, 255, 255]),
    ("magenta", [255, 0, 255]),
    ("fuchsia", [255, 0, 255]),
    ("gray", [128, 128, 128]),
    ("grey", [128, 128, 128]),
    ("silver", [192, 192, 192]),
    ("orange", [255, 165, 0]),
    ("navy", [0, 0, 128]),
    ("maroon", [128, 0, 0]),
];

/// A colour with the opacity its notation carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Colour {
    pub(super) rgb: [u8; 3],
    pub(super) alpha: f32,
}

/// What a `fill` or `stroke` property names.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Ink {
    None,
    Colour(Colour),
}

pub(super) enum Parsed {
    Ink(Ink),
    Reference(String),
    Unknown,
}

fn hex(text: &str) -> Option<Colour> {
    let digits: Vec<u8> = text
        .bytes()
        .map(|b| {
            char::from(b)
                .to_digit(16)
                .and_then(|d| u8::try_from(d).ok())
        })
        .collect::<Option<_>>()?;
    let pair = |at: usize| -> Option<u8> {
        digits
            .get(at)?
            .checked_mul(16)?
            .checked_add(*digits.get(at + 1)?)
    };
    let single = |at: usize| digits.get(at).map(|d| d * 17);
    let (rgb, alpha) = match digits.len() {
        3 => ([single(0)?, single(1)?, single(2)?], 255),
        4 => ([single(0)?, single(1)?, single(2)?], single(3)?),
        6 => ([pair(0)?, pair(2)?, pair(4)?], 255),
        8 => ([pair(0)?, pair(2)?, pair(4)?], pair(6)?),
        _ => return None,
    };
    Some(Colour {
        rgb,
        alpha: f32::from(alpha) / 255.0,
    })
}

fn function(text: &str) -> Option<Colour> {
    let (name, rest) = text.split_once('(')?;
    let inner = rest.strip_suffix(')')?;
    let parts: Vec<&str> = inner
        .split([',', ' ', '/'])
        .filter(|part| !part.is_empty())
        .collect();
    let channel = |part: &str| -> Option<u8> {
        let value = leading_number(part)?;
        let value = if part.trim().ends_with('%') {
            value * 2.55
        } else {
            value
        };
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the value is clamped to a byte"
        )]
        let byte = value.clamp(0.0, 255.0).round() as u8;
        Some(byte)
    };
    if !matches!(name.trim(), "rgb" | "rgba") || parts.len() < 3 {
        return None;
    }
    let alpha = parts
        .get(3)
        .and_then(|part| leading_number(part))
        .map_or(1.0, |a| {
            if parts.get(3).is_some_and(|part| part.ends_with('%')) {
                a / 100.0
            } else {
                a
            }
        });
    Some(Colour {
        rgb: [
            channel(parts.first()?)?,
            channel(parts.get(1)?)?,
            channel(parts.get(2)?)?,
        ],
        alpha: alpha.clamp(0.0, 1.0),
    })
}

pub(super) fn colour(text: &str) -> Option<Colour> {
    let text = text.trim();
    if let Some(digits) = text.strip_prefix('#') {
        return hex(digits);
    }
    if text.contains('(') {
        return function(text);
    }
    let lower = text.to_ascii_lowercase();
    NAMED
        .iter()
        .find(|(name, _)| *name == lower)
        .map(|(_, rgb)| Colour {
            rgb: *rgb,
            alpha: 1.0,
        })
}

pub(super) fn ink(text: &str) -> Parsed {
    let text = text.trim();
    if text.eq_ignore_ascii_case("none") || text.eq_ignore_ascii_case("transparent") {
        return Parsed::Ink(Ink::None);
    }
    if text.eq_ignore_ascii_case("currentcolor") {
        return Parsed::Ink(Ink::Colour(Colour {
            rgb: [0, 0, 0],
            alpha: 1.0,
        }));
    }
    if let Some(inner) = text.strip_prefix("url(") {
        let id = inner
            .split(')')
            .next()
            .map(|id| id.trim().trim_matches(['"', '\'']).trim_start_matches('#'));
        return id.map_or(Parsed::Unknown, |id| Parsed::Reference(id.to_owned()));
    }
    colour(text).map_or(Parsed::Unknown, |colour| Parsed::Ink(Ink::Colour(colour)))
}

/// A palette slot an element names in its `id`, as `colorstyle:Bright_opacity:40`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Slot {
    pub(crate) name: String,
    pub(crate) opacity: f32,
}

const SLOT_PREFIX: &str = "colorstyle:";

/// Reads the slot an element `id` names.
pub(super) fn slot_of(id: &str) -> Option<Slot> {
    let rest = id.strip_prefix(SLOT_PREFIX)?;
    let name = rest.split('_').next().filter(|name| !name.is_empty())?;
    let opacity = rest
        .split_once("opacity:")
        .and_then(|(_, tail)| leading_number(tail.split('_').next().unwrap_or_default()))
        .map_or(1.0, |percent| (percent / 100.0).clamp(0.0, 1.0));
    Some(Slot {
        name: name.to_owned(),
        opacity,
    })
}

/// The presentation properties of an element after inheritance.
#[derive(Debug, Clone)]
pub(super) struct Style {
    pub(super) slot: Option<Slot>,
    pub(super) fill: Ink,
    pub(super) stroke: Ink,
    pub(super) stroke_width: f32,
    pub(super) fill_opacity: f32,
    pub(super) stroke_opacity: f32,
    pub(super) opacity: f32,
    pub(super) cap: Cap,
    pub(super) join: Join,
    pub(super) miter_limit: f32,
    pub(super) hidden: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            slot: None,
            fill: Ink::Colour(Colour {
                rgb: [0, 0, 0],
                alpha: 1.0,
            }),
            stroke: Ink::None,
            stroke_width: 1.0,
            fill_opacity: 1.0,
            stroke_opacity: 1.0,
            opacity: 1.0,
            cap: Cap::Butt,
            join: Join::Miter,
            miter_limit: 4.0,
            hidden: false,
        }
    }
}

impl Style {
    /// Applies one property; `gradient` resolves the id of a paint server to its flat colour.
    pub(super) fn set(
        &mut self,
        name: &str,
        value: &str,
        gradient: &dyn Fn(&str) -> Option<Colour>,
        unsupported: &mut dyn FnMut(&'static str),
    ) {
        let value = value.trim();
        let opacity = |text: &str| leading_number(text).map(|n| n.clamp(0.0, 1.0));
        let resolve = |parsed: Parsed, unsupported: &mut dyn FnMut(&'static str)| match parsed {
            Parsed::Ink(ink) => Some(ink),
            Parsed::Reference(id) => {
                unsupported("paint servers are drawn as their first colour");
                gradient(&id).map(Ink::Colour)
            }
            Parsed::Unknown => None,
        };
        match name {
            "fill" => self.fill = resolve(ink(value), unsupported).unwrap_or(self.fill),
            "stroke" => self.stroke = resolve(ink(value), unsupported).unwrap_or(self.stroke),
            "stroke-width" => {
                if let Some(width) = leading_number(value).filter(|w| *w >= 0.0) {
                    self.stroke_width = width;
                }
            }
            "fill-opacity" => self.fill_opacity = opacity(value).unwrap_or(self.fill_opacity),
            "stroke-opacity" => {
                self.stroke_opacity = opacity(value).unwrap_or(self.stroke_opacity);
            }
            "opacity" => self.opacity *= opacity(value).unwrap_or(1.0),
            "stroke-linecap" => {
                self.cap = match value {
                    "round" => Cap::Round,
                    "square" => Cap::Square,
                    _ => Cap::Butt,
                };
            }
            "stroke-linejoin" => {
                self.join = match value {
                    "round" => Join::Round,
                    "bevel" => Join::Bevel,
                    _ => Join::Miter,
                };
            }
            "stroke-miterlimit" => {
                if let Some(limit) = leading_number(value).filter(|l| *l >= 1.0) {
                    self.miter_limit = limit;
                }
            }
            "display" => self.hidden = value == "none",
            "visibility" => self.hidden |= matches!(value, "hidden" | "collapse"),
            _ => {}
        }
    }
}
