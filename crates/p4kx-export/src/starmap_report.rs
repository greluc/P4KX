use std::sync::Arc;

use p4kx_extraction::Guid;

/// Where one Starmap Record is placed: the first Child element that names it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Placement {
    pub(crate) system: Arc<str>,
    pub(crate) position: [f64; 3],
    pub(crate) entity_name: String,
    pub(crate) placed_under: Option<Guid>,
}

/// What reading the system containers found, besides the placements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ContainerCounts {
    pub(crate) systems: usize,
    pub(crate) unreadable: usize,
    pub(crate) missing: usize,
    pub(crate) malformed: usize,
    pub(crate) ambiguous: usize,
}

/// What reading the system containers saw, for a person to check the placements by.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StarmapReport {
    /// One line per container read that names starmap records, up to 300 per star system, then
    /// a line with the total: its path, the Child elements below it (nested external containers
    /// included), those naming a starmap record and those naming an external container.
    pub containers: Vec<String>,
    /// One line per container, placement or limit that left objects without a position, with
    /// the reason, up to 300 per star system. A missing or unreadable container is counted once
    /// per Child that names it, not once per distinct container.
    pub issues: Vec<String>,
    /// One line per record that more than one Child element places, up to 100.
    pub repeated: Vec<String>,
    /// The socpak containers below ObjectContainers/PU/system that no Child names, up to 200,
    /// the last line giving the total.
    pub unreferenced: Vec<String>,
    /// The longest chain of external containers the walk followed, counted from the system
    /// container (0); the limit is 32.
    pub deepest_external: usize,
    /// How long reading the system containers took.
    pub elapsed: std::time::Duration,
}

/// Placements of one record closer than this, in metres, are one place.
const SAME_PLACE: f64 = 1.0;

/// The longest quoted attribute value in a report line.
const QUOTE_LEN: usize = 64;

/// Returns `value` quoted for a report line, cut to 64 characters.
pub(crate) fn clip(value: &str) -> String {
    let mut quoted: String = value.chars().take(QUOTE_LEN).collect();
    if value.chars().count() > QUOTE_LEN {
        quoted.push('…');
    }
    format!("{quoted:?}")
}

pub(crate) fn apart(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
        > SAME_PLACE
}
