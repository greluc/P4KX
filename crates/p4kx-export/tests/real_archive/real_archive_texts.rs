// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The text check of the verification, per kind: a key a language's `global.ini` holds resolves in
//! the catalog, a key it lacks leaves that language `null`, and the keys a file lacks are counted
//! against a baseline. The translation completeness of the game is measured, never asserted as a
//! global share.

use std::path::Path;

use p4kx_export::{Kind, Language, PLACEHOLDER_KEYS, TextSet};
use p4kx_extraction::{Archive, LocalizationSet};
use serde_json::Value;

use crate::real_archive_verification::{TestResult, list, print_line};

/// The baseline of a kind: the largest share of its keyed texts the English and the German
/// `global.ini` may lack, as measured on LIVE and rounded up to the hundredth. A kind with keyed
/// texts that is not listed has a baseline of zero, so a new kind with untranslated keys names
/// itself.
#[derive(Clone, Copy)]
struct Baseline {
    kind: Kind,
    english: f64,
    german: f64,
}

const fn baseline_of(kind: Kind, english: f64, german: f64) -> Baseline {
    Baseline {
        kind,
        english,
        german,
    }
}

const BASELINES: [Baseline; 10] = [
    baseline_of(Kind::Manufacturers, 0.01, 0.06),
    baseline_of(Kind::Items, 0.01, 0.49),
    baseline_of(Kind::Ships, 0.01, 0.31),
    baseline_of(Kind::Commodities, 0.0, 0.22),
    baseline_of(Kind::Blueprints, 0.0, 1.0),
    baseline_of(Kind::GameplayProperties, 0.0, 0.97),
    baseline_of(Kind::StarmapObjects, 0.04, 0.67),
    baseline_of(Kind::Contracts, 0.01, 1.0),
    baseline_of(Kind::MissionLocationTemplates, 0.09, 1.0),
    baseline_of(Kind::ResourceGroups, 0.0, 0.0),
];

/// What the catalog holds for the keyed texts of one kind in one language.
#[derive(Debug, Default)]
struct Coverage {
    /// Texts whose language file holds the key and the catalog has the text.
    resolved: usize,
    /// Texts whose language file lacks the key and the catalog has `null`.
    fallback: usize,
    /// Texts where the catalog contradicts the file: the exporter is wrong.
    contradicted: Vec<String>,
}

impl Coverage {
    fn count(&mut self, texts: &TextSet<'_>, language: Language, key: &str, shown: Option<&Value>) {
        let translated = texts.is_translated(key, language);
        match (translated, shown.is_some_and(Value::is_string)) {
            (true, true) => self.resolved += 1,
            (false, false) => self.fallback += 1,
            _ => {
                if self.contradicted.len() < 5 {
                    self.contradicted.push(key.to_owned());
                }
            }
        }
    }

    const fn total(&self) -> usize {
        self.resolved + self.fallback + self.contradicted.len()
    }
}

fn is_placeholder(key: &str) -> bool {
    PLACEHOLDER_KEYS.iter().any(|placeholder| {
        placeholder
            .trim_start_matches('@')
            .eq_ignore_ascii_case(key.trim_start_matches('@'))
    })
}

/// Returns the keyed texts of `entity`: its `name` and `description` and, for a location
/// template, the `displayName` of each of its `names`.
fn keyed_texts(entity: &Value) -> impl Iterator<Item = (&str, &Value)> {
    let own = ["name", "description"]
        .into_iter()
        .filter_map(|field| entity.get(field));
    let variants = entity
        .get("names")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|variant| variant.get("displayName"));
    own.chain(variants).filter_map(|text| {
        let key = text.get("key")?.as_str()?;
        (!is_placeholder(key)).then_some((key, text))
    })
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a share of counts far below 2^52"
)]
fn share(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

fn baseline(kind: Kind) -> Baseline {
    BASELINES
        .into_iter()
        .find(|known| known.kind == kind)
        .unwrap_or_else(|| baseline_of(kind, 0.0, 0.0))
}

/// Checks the keyed texts of every kind against the `global.ini` files of the Archive at `path`
/// and prints the numbers per kind.
pub(crate) fn check_texts(path: &Path, catalog: &Value) -> TestResult {
    let archive = Archive::open_with_key(path, Some(p4kx_testkit::real_archive_key()?))?;
    let set = LocalizationSet::read(&archive, &[Language::English, Language::German])?;
    let texts = set.index()?;
    for language in [Language::English, Language::German] {
        assert!(
            texts.ini(language).is_some(),
            "the Archive holds {language}"
        );
    }
    let mut failures: Vec<String> = Vec::new();
    for kind in Kind::ALL {
        let (mut english, mut german) = (Coverage::default(), Coverage::default());
        for entity in list(catalog, kind)? {
            for (key, text) in keyed_texts(entity) {
                english.count(&texts, Language::English, key, text.get("en"));
                german.count(&texts, Language::German, key, text.get("de"));
            }
        }
        let keyed = english.total();
        if keyed == 0 {
            continue;
        }
        let baseline = baseline(kind);
        print_line(&format!(
            "{kind}: {keyed} keyed texts, en {} resolved / {} unresolved ({:.3}, baseline {:.2}), de {} resolved / {} fall back to en ({:.3}, baseline {:.2})",
            english.resolved,
            english.fallback,
            share(english.fallback, keyed),
            baseline.english,
            german.resolved,
            german.fallback,
            share(german.fallback, keyed),
            baseline.german,
        ));
        verdict(kind, "en", &english, baseline.english, &mut failures);
        verdict(kind, "de", &german, baseline.german, &mut failures);
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n").into())
    }
}

fn verdict(
    kind: Kind,
    language: &str,
    coverage: &Coverage,
    ceiling: f64,
    failures: &mut Vec<String>,
) {
    if !coverage.contradicted.is_empty() {
        failures.push(format!(
            "{kind}: the {language} text contradicts the global.ini for keys such as {:?}",
            coverage.contradicted
        ));
    }
    let fallback = share(coverage.fallback, coverage.total());
    if fallback > ceiling {
        failures.push(format!(
            "{kind}: {fallback:.3} of the {language} texts fall back, above the baseline {ceiling:.3}"
        ));
    }
}
