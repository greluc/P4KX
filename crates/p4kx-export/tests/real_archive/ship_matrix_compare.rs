// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Compares the `playability` of the catalog ships with a ship matrix saved by hand; a
//! development check, never an export source (ADR-0245).
//!
//! A matrix entry maps to a catalog ship through the curated `shipStoreSlugs`: first by the RSI
//! ship id, then by the last segment of the entry's `url` against the slug, ignoring case.

use std::collections::{BTreeSet, HashMap};

use serde::Deserialize;
use serde_json::Value;

/// The `production_status` that stands for a ship players can fly.
pub(crate) const FLIGHT_READY: &str = "flight-ready";

/// One entry of the ship matrix; only the fields the check reads.
#[derive(Debug, Deserialize)]
pub(crate) struct MatrixShip {
    #[serde(default)]
    id: Option<Value>,
    pub(crate) name: String,
    #[serde(default)]
    production_status: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Matrix {
    Wrapped { data: Vec<MatrixShip> },
    Bare(Vec<MatrixShip>),
}

impl MatrixShip {
    fn rsi_id(&self) -> Option<u32> {
        match self.id.as_ref()? {
            Value::Number(number) => number.as_u64().and_then(|id| u32::try_from(id).ok()),
            Value::String(text) => text.trim().parse().ok(),
            _ => None,
        }
    }

    /// The last path segment of `url`, without a trailing slash.
    fn slug(&self) -> Option<&str> {
        self.url
            .as_deref()?
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .filter(|segment| !segment.is_empty())
    }

    fn status(&self) -> &str {
        self.production_status.as_deref().unwrap_or("no status")
    }

    fn is_flight_ready(&self) -> bool {
        self.production_status.as_deref() == Some(FLIGHT_READY)
    }
}

/// Parses the ship matrix JSON: the `{"data": [...]}` envelope of the RSI endpoint or a bare list.
///
/// # Errors
///
/// Returns the parse error when the text is neither.
pub(crate) fn parse_matrix(text: &str) -> Result<Vec<MatrixShip>, serde_json::Error> {
    Ok(match serde_json::from_str(text)? {
        Matrix::Wrapped { data } | Matrix::Bare(data) => data,
    })
}

/// A catalog ship: its `className` and `playability`, `None` for `null`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CatalogShip<'a> {
    pub(crate) class_name: &'a str,
    pub(crate) playability: Option<&'a str>,
}

/// A curated store slug and the class it maps to.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StoreSlug<'a> {
    pub(crate) slug: &'a str,
    pub(crate) rsi_id: u32,
    pub(crate) class_name: Option<&'a str>,
}

/// A ship the matrix and the catalog disagree on, or a `null` ship with the matrix verdict.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Finding {
    pub(crate) class_name: String,
    pub(crate) playability: String,
    pub(crate) matrix: String,
}

/// The result of one comparison.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Report {
    /// Flight ready in the matrix, not `player` in the catalog.
    pub(crate) flight_ready_not_player: Vec<Finding>,
    /// Flight-ready matrix names that map to no catalog class.
    pub(crate) flight_ready_unmapped: Vec<String>,
    /// `player` in the catalog, unknown to the matrix or not flight ready there.
    pub(crate) player_not_flight_ready: Vec<Finding>,
    /// Ships with `playability` `null`, with what the matrix says.
    pub(crate) null_ships: Vec<Finding>,
}

impl Report {
    /// The report as text lines, one finding per line.
    #[must_use]
    pub(crate) fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let sections = [
            (
                "flight ready in the matrix, not player",
                &self.flight_ready_not_player,
            ),
            (
                "player, not flight ready in the matrix",
                &self.player_not_flight_ready,
            ),
            ("playability null", &self.null_ships),
        ];
        for (title, findings) in sections {
            lines.push(format!("{title}: {}", findings.len()));
            lines.extend(findings.iter().map(|finding| {
                format!(
                    "  {} [{}] matrix: {}",
                    finding.class_name, finding.playability, finding.matrix
                )
            }));
        }
        lines.push(format!(
            "flight ready in the matrix, no class mapped: {}",
            self.flight_ready_unmapped.len()
        ));
        lines.extend(
            self.flight_ready_unmapped
                .iter()
                .map(|name| format!("  {name}")),
        );
        lines
    }
}

const UNKNOWN: &str = "unknown to the matrix";

/// Matrix names the curated store slugs do not map, with the catalog class they stand for; read
/// off the Archive's class names, used only by this check.
const NAME_SUPPLEMENT: &[(&str, &str)] = &[
    ("Constellation Mk IV Aquila", "RSI_Constellation_Aquila"),
    (
        "Constellation Mk IV Andromeda",
        "RSI_Constellation_Andromeda",
    ),
    ("Constellation Mk IV Taurus", "RSI_Constellation_Taurus"),
    ("Constellation Mk IV Phoenix", "RSI_Constellation_Phoenix"),
    (
        "Constellation Mk V Centaurus",
        "RSI_Constellation_Mk5_Centaurus",
    ),
    ("Carrack w/C8X", "ANVL_Carrack"),
    ("Carrack Expedition w/C8X", "ANVL_Carrack_Expedition"),
    ("Sabre Raven EX", "AEGS_Sabre_Raven_EX"),
    ("Dragonfly Black", "DRAK_Dragonfly"),
    ("600i Explorer", "ORIG_600i"),
    ("Hammerhead", "AEGS_Hammerhead"),
    ("Argo Mole Carbon Edition", "ARGO_MOLE"),
    ("Argo Mole Talus Edition", "ARGO_MOLE"),
    ("Railen", "GAMA_Railen"),
    ("Tyilui", "GAMA_Tyilui"),
    ("Basher", "GLSN_Basher"),
    ("S-65 Stingray", "KRIG_S65_Stingray"),
    ("Skylark", "RSI_Skylark"),
];

fn class_of<'a>(
    ship: &MatrixShip,
    by_id: &HashMap<u32, &'a StoreSlug<'a>>,
    by_slug: &HashMap<String, &'a StoreSlug<'a>>,
) -> Option<&'a str> {
    let by_rsi_id = ship.rsi_id().and_then(|id| by_id.get(&id));
    let found = by_rsi_id.or_else(|| {
        ship.slug()
            .and_then(|slug| by_slug.get(&slug.to_lowercase()))
    });
    found.and_then(|entry| entry.class_name).or_else(|| {
        NAME_SUPPLEMENT
            .iter()
            .find_map(|(name, class)| (*name == ship.name).then_some(*class))
    })
}

/// The classes of `NAME_SUPPLEMENT` that are not among the catalog `ships`: their matrix names
/// would silently count as unmapped.
#[must_use]
pub(crate) fn supplement_gaps(ships: &[CatalogShip<'_>]) -> Vec<&'static str> {
    NAME_SUPPLEMENT
        .iter()
        .map(|(_, class)| *class)
        .filter(|class| ships.iter().all(|ship| ship.class_name != *class))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Compares the catalog `ships` with the `matrix`, mapping through `slugs`.
///
/// A catalog class counts as flight ready when any of its matrix entries is, so a flight-ready
/// variant mapped to the same class (Mole editions, Carrack w/C8X) hides a base entry that is not.
#[must_use]
pub(crate) fn compare(
    ships: &[CatalogShip<'_>],
    slugs: &[StoreSlug<'_>],
    matrix: &[MatrixShip],
) -> Report {
    let by_id: HashMap<u32, &StoreSlug<'_>> =
        slugs.iter().map(|slug| (slug.rsi_id, slug)).collect();
    let by_slug: HashMap<String, &StoreSlug<'_>> = slugs
        .iter()
        .map(|slug| (slug.slug.to_lowercase(), slug))
        .collect();
    let mut by_class: HashMap<&str, Vec<&MatrixShip>> = HashMap::new();
    let mut report = Report::default();
    for ship in matrix {
        match class_of(ship, &by_id, &by_slug) {
            Some(class) => by_class.entry(class).or_default().push(ship),
            None if ship.is_flight_ready() => report.flight_ready_unmapped.push(ship.name.clone()),
            None => {}
        }
    }
    for ship in ships {
        let entries = by_class.get(ship.class_name).map_or(&[][..], Vec::as_slice);
        let verdict = verdict(entries);
        let finding = || Finding {
            class_name: ship.class_name.to_owned(),
            playability: ship.playability.unwrap_or("null").to_owned(),
            matrix: verdict.clone(),
        };
        let ready = entries.iter().any(|entry| entry.is_flight_ready());
        match ship.playability {
            Some("player") if !ready => report.player_not_flight_ready.push(finding()),
            Some("player") => {}
            None => report.null_ships.push(finding()),
            Some(_) if ready => report.flight_ready_not_player.push(finding()),
            Some(_) => {}
        }
    }
    report
}

fn verdict(entries: &[&MatrixShip]) -> String {
    if entries.is_empty() {
        return UNKNOWN.to_owned();
    }
    let parts: Vec<String> = entries
        .iter()
        .map(|entry| format!("{} ({})", entry.name, entry.status()))
        .collect();
    parts.join(", ")
}

#[cfg(test)]
mod tests;
