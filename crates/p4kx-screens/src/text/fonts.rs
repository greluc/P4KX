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
    choice: Box<str>,
}

const SEPARATOR: char = '\u{1f}';

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
        let stem = normalise(stem);
        let bold = flag(root, "isBold").unwrap_or(false);
        let choice = choice_key(&stem, &families, &weight, bold);
        Some(Self {
            name: name.to_owned(),
            stem,
            families,
            weight,
            bold,
            scale: float(root, "scaleModifier")
                .filter(|scale| scale.is_finite() && *scale > 0.0)
                .unwrap_or(1.0),
            choice,
        })
    }
}

/// Joins everything [`score`] reads of a style, so equal keys choose the same font.
fn choice_key(stem: &str, families: &[String], weight: &str, bold: bool) -> Box<str> {
    let mut key = String::with_capacity(stem.len() + weight.len() + 8);
    key.push(if bold { 'b' } else { 'r' });
    for part in std::iter::once(stem)
        .chain(families.iter().map(String::as_str))
        .chain(std::iter::once(weight))
    {
        key.push(SEPARATOR);
        key.push_str(part);
    }
    key.into_boxed_str()
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
    chosen: HashMap<Box<str>, Option<usize>>,
    default: OnceCell<Option<usize>>,
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
            default: OnceCell::new(),
        }
    }

    pub(crate) fn face(&self, at: usize) -> Option<&FontFace<'t>> {
        let font = self.fonts.get(at)?;
        Some(self.faces.get(at)?.get_or_init(|| FontFace::new(font)))
    }

    fn drawable(&self, at: usize) -> bool {
        self.face(at).is_some_and(FontFace::is_drawable)
    }

    pub(crate) fn default_font(&self) -> Option<usize> {
        *self.default.get_or_init(|| {
            self.best(|names, _| {
                names
                    .iter()
                    .any(|name| name == DEFAULT_FONT)
                    .then_some(EXACT)
            })
        })
    }

    pub(crate) fn pick(&mut self, style: &FontStyleInfo) -> Option<usize> {
        if let Some(found) = self.chosen.get(&*style.choice) {
            return *found;
        }
        let found = self.best(|names, font| {
            names
                .iter()
                .filter_map(|name| score(name, style, font))
                .max()
        });
        self.chosen.insert(style.choice.clone(), found);
        found
    }

    fn best(&self, rate: impl Fn(&[String], &Font) -> Option<u32>) -> Option<usize> {
        let mut best: Option<(u32, usize)> = None;
        for (at, font) in self.fonts.iter().enumerate() {
            let Some(points) = self.names.get(at).and_then(|names| rate(names, font)) else {
                continue;
            };
            if self.drawable(at) && best.is_none_or(|(top, _)| points > top) {
                best = Some((points, at));
            }
        }
        best.map(|(_, at)| at)
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
    use super::{choice_key, split_weight};

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn styles_that_score_differently_get_different_keys() {
        let base = choice_key("paint", &names(&["paint"]), "bold", false);
        assert_eq!(base, choice_key("paint", &names(&["paint"]), "bold", false));
        assert_ne!(base, choice_key("paint", &names(&["paint"]), "bold", true));
        assert_ne!(
            base,
            choice_key("paint", &names(&["paint"]), "light", false)
        );
        assert_ne!(
            base,
            choice_key("paint", &names(&["paint", "name"]), "bold", false)
        );
        assert_ne!(base, choice_key("other", &names(&["paint"]), "bold", false));
    }

    #[test]
    fn a_family_is_not_mistaken_for_a_weight() {
        assert_ne!(
            choice_key("s", &names(&["a", "b"]), "", false),
            choice_key("s", &names(&["a"]), "b", false)
        );
    }

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
