// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;

use p4kx_dataforge::{DataForge, Guid};
use p4kx_gfx::{Font, FontFace};

use crate::reader::{flag, float, text};

const DEFAULT_FONT: &str = "blenderpromedium";
const EXACT: u32 = 100;
const FAMILY: u32 = 50;
const PREFIX: u32 = 20;
const BOLD_MATCH: u32 = 10;
const WEIGHT_MATCH: u32 = 5;

/// What a `BuildingBlocks_FontStyle` Record says about the font to draw with.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FontStyleInfo {
    pub(crate) name: String,
    stem: String,
    families: Vec<String>,
    weight: String,
    bold: bool,
    pub(crate) scale: f32,
}

const WEIGHTS: [&str; 8] = [
    "semibold", "regular", "bold", "light", "medium", "book", "thin", "heavy",
];

fn split_weight(name: &str) -> (String, String) {
    if let Some((family, weight)) = name.split_once(['-', ' ']) {
        return (normalise(family), normalise(weight));
    }
    let name = normalise(name);
    WEIGHTS
        .iter()
        .find_map(|weight| {
            name.strip_suffix(weight)
                .filter(|family| !family.is_empty())
                .map(|family| (family.to_owned(), (*weight).to_owned()))
        })
        .unwrap_or((name, String::new()))
}

impl FontStyleInfo {
    fn read(forge: &DataForge<'_>, guid: Guid) -> Option<Self> {
        let record = forge.record_by_guid(guid)?;
        let tree = record.tree().ok()?;
        let root = tree.root();
        let paint = text(root, "paintFile")?;
        let file = paint.rsplit(['/', '\\']).next().unwrap_or(paint);
        let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
        let name = tree.record_path().rsplit('/').next().unwrap_or_default();
        let (family, weight) = split_weight(stem);
        let mut families = vec![family];
        let (alternative, _) = split_weight(name);
        if !families.contains(&alternative) {
            families.push(alternative);
        }
        Some(Self {
            name: name.to_owned(),
            stem: normalise(stem),
            families,
            weight,
            bold: flag(root, "isBold").unwrap_or(false),
            scale: float(root, "scaleModifier")
                .filter(|scale| scale.is_finite() && *scale > 0.0)
                .unwrap_or(1.0),
        })
    }
}

fn normalise(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

/// The font styles read from the Records, once each.
#[derive(Debug, Default)]
pub(crate) struct FontStyles {
    cache: HashMap<Guid, Option<Rc<FontStyleInfo>>>,
}

impl FontStyles {
    pub(crate) fn get(&mut self, forge: &DataForge<'_>, guid: Guid) -> Option<Rc<FontStyleInfo>> {
        self.cache
            .entry(guid)
            .or_insert_with(|| FontStyleInfo::read(forge, guid).map(Rc::new))
            .clone()
    }
}

/// The fonts of the movie `fonts_en.gfx`, looked up by the font style of a widget.
#[derive(Debug)]
pub(crate) struct FontBook<'t> {
    fonts: &'t [Font],
    names: Vec<Vec<String>>,
    faces: Vec<OnceCell<FontFace<'t>>>,
    chosen: HashMap<(String, bool), Option<usize>>,
}

impl<'t> FontBook<'t> {
    pub(crate) fn new(fonts: &'t [Font]) -> Self {
        let names = fonts
            .iter()
            .map(|font| {
                std::iter::once(&font.name)
                    .chain(font.full_name.as_ref())
                    .chain(font.export_name.as_ref())
                    .map(|name| normalise(name))
                    .filter(|name| !name.is_empty())
                    .collect()
            })
            .collect();
        Self {
            fonts,
            names,
            faces: fonts.iter().map(|_| OnceCell::new()).collect(),
            chosen: HashMap::new(),
        }
    }

    pub(crate) fn face(&self, at: usize) -> Option<&FontFace<'t>> {
        let font = self.fonts.get(at)?;
        Some(self.faces.get(at)?.get_or_init(|| FontFace::new(font)))
    }

    fn drawable(&self, at: usize) -> bool {
        self.face(at).is_some_and(FontFace::is_drawable)
    }

    pub(crate) fn default_font(&mut self) -> Option<usize> {
        self.best((DEFAULT_FONT.to_owned(), false), |names, _| {
            names
                .iter()
                .any(|name| name == DEFAULT_FONT)
                .then_some(EXACT)
        })
    }

    pub(crate) fn pick(&mut self, style: &FontStyleInfo) -> Option<usize> {
        self.best((style.stem.clone(), style.bold), |names, font| {
            names
                .iter()
                .filter_map(|name| score(name, style, font))
                .max()
        })
    }

    fn best(
        &mut self,
        key: (String, bool),
        rate: impl Fn(&[String], &Font) -> Option<u32>,
    ) -> Option<usize> {
        if let Some(found) = self.chosen.get(&key) {
            return *found;
        }
        let mut best: Option<(u32, usize)> = None;
        for (at, font) in self.fonts.iter().enumerate() {
            let Some(points) = self.names.get(at).and_then(|names| rate(names, font)) else {
                continue;
            };
            if self.drawable(at) && best.is_none_or(|(top, _)| points > top) {
                best = Some((points, at));
            }
        }
        let found = best.map(|(_, at)| at);
        self.chosen.insert(key, found);
        found
    }
}

fn score(name: &str, style: &FontStyleInfo, font: &Font) -> Option<u32> {
    let mut points = if name == style.stem {
        EXACT
    } else if style.families.iter().any(|family| name == family) {
        FAMILY
    } else if style.families.iter().any(|family| name.starts_with(family)) {
        PREFIX
    } else {
        return None;
    };
    if font.bold == style.bold {
        points += BOLD_MATCH;
    }
    if !style.weight.is_empty() && name.contains(&style.weight) {
        points += WEIGHT_MATCH;
    }
    Some(points)
}

#[cfg(test)]
mod tests {
    use super::split_weight;

    fn split(name: &str) -> (String, String) {
        split_weight(name)
    }

    #[test]
    fn the_weight_follows_a_dash_or_a_space() {
        assert_eq!(
            split("BlenderPro-Bold"),
            ("blenderpro".into(), "bold".into())
        );
        assert_eq!(
            split("Aldrich regular"),
            ("aldrich".into(), "regular".into())
        );
    }

    #[test]
    fn a_weight_may_be_the_end_of_the_name() {
        assert_eq!(
            split("CIGOpticalRegular"),
            ("cigoptical".into(), "regular".into())
        );
        assert_eq!(split("Furore"), ("furore".into(), String::new()));
        assert_eq!(split("AUDIMB_"), ("audimb".into(), String::new()));
    }
}
