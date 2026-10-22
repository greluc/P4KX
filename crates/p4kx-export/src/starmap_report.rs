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
    /// One line per container read: its path, the Child elements below it (nested external
    /// containers included), those naming a starmap record and those naming an external container.
    pub containers: Vec<String>,
    /// One line per container or placement that could not be used, with the reason.
    pub issues: Vec<String>,
    /// One line per record that more than one Child element places, up to 100.
    pub repeated: Vec<String>,
    /// The raw `rot` values of the first Children with a rotation other than the identity, up to
    /// 40, each with its container and entity name.
    pub rotations: Vec<String>,
    /// The socpak containers below ObjectContainers/PU/system that no Child names, up to 200,
    /// the last line giving the total.
    pub unreferenced: Vec<String>,
}

/// Placements of one record closer than this, in metres, are one place.
const SAME_PLACE: f64 = 1.0;

pub(crate) fn apart(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
        > SAME_PLACE
}
