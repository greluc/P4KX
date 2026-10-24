// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_assembly::{EntityNode, LoadoutEntry};
use p4kx_extraction::{DataForge, Guid, RecordTree, StructValue, TreeBuilder};

use super::model::{Cargo, CargoGrid, Source, SourceKind};
use crate::record_fields::{number, structure};

const GRID_TYPE: &str = "CargoGrid";
const TEMPLATE_SUFFIX: &str = "_template";
const CUBIC_METRES_PER_SCU: f64 = 1.953_125;
const CELL_METRES: f64 = 1.25;
const CELL_TOLERANCE: f64 = 1e-4;
const MAX_SIDE_METRES: f64 = 1000.0;
const MAX_CLOSED_SCU: f64 = 1e9;

/// What an `InventoryContainer` Record states of its capacity.
#[derive(Debug, Clone)]
struct Container {
    record_path: String,
    shape: Shape,
}

#[derive(Debug, Clone)]
enum Shape {
    Open { dimensions: [f64; 3] },
    Closed { scu: f64, field: &'static str },
    Unknown(String),
}

/// Finds the cargo grids of default Loadouts and reads their capacity, keeping each
/// `InventoryContainer` Record it has read.
pub(crate) struct CargoReader<'f> {
    forge: &'f DataForge<'f>,
    containers: HashMap<Guid, Result<Container, String>>,
}

impl<'f> CargoReader<'f> {
    pub(crate) fn new(forge: &'f DataForge<'f>) -> Self {
        Self {
            forge,
            containers: HashMap::new(),
        }
    }

    /// Returns the cargo grids among the Loadout `entries` of a ship and their sum. Grids of
    /// docked vehicles are not in the tree, templates are left out.
    pub(crate) fn read(
        &mut self,
        entries: &[LoadoutEntry<'_>],
        trees: &mut TreeBuilder<'f>,
    ) -> Cargo {
        let mut grids = Vec::new();
        for entry in entries {
            if let Some(node) = entry.entity()
                && is_grid(node)
            {
                grids.push(self.grid(entry, node, trees));
            }
        }
        let with_issue = grids.iter().filter(|grid| grid.scu.is_none()).count();
        let known = grids.is_empty() || grids.len() > with_issue;
        let scu = grids.iter().filter_map(|grid| grid.scu).sum::<f64>();
        let cells = grids
            .iter()
            .filter_map(|grid| grid.cells)
            .fold(0_u64, u64::saturating_add);
        Cargo {
            scu: known.then(|| round(scu)),
            cells: known.then_some(cells),
            grids_with_issue: with_issue,
            grids,
        }
    }

    fn grid(
        &mut self,
        entry: &LoadoutEntry<'_>,
        node: &EntityNode,
        trees: &mut TreeBuilder<'f>,
    ) -> CargoGrid {
        let mut grid = CargoGrid {
            port: entry.path().to_owned(),
            class_name: node.class_name().to_owned(),
            guid: node.guid(),
            container: node.inventory_container(),
            shape: None,
            scu: None,
            cells: None,
            dimensions: None,
            source: None,
            issue: None,
        };
        let Some(container) = grid.container else {
            grid.issue = Some("the Entity names no inventory container".to_owned());
            return grid;
        };
        match self.container(container, trees) {
            Err(reason) => grid.issue = Some(reason.clone()),
            Ok(read) => fill(&mut grid, read),
        }
        grid
    }

    fn container(&mut self, guid: Guid, trees: &mut TreeBuilder<'f>) -> &Result<Container, String> {
        let forge = self.forge;
        self.containers.entry(guid).or_insert_with(|| {
            let record = forge
                .record_by_guid(guid)
                .ok_or_else(|| format!("the inventory container {guid} does not exist: a dangling Reference in the game data"))?;
            let tree = trees
                .build(&record)
                .map_err(|error| format!("cannot read the inventory container {guid}: {error}"))?;
            Ok(read_container(&tree))
        })
    }
}

fn is_grid(node: &EntityNode) -> bool {
    node.attach_type().eq_ignore_ascii_case(GRID_TYPE)
        && !node
            .class_name()
            .to_ascii_lowercase()
            .ends_with(TEMPLATE_SUFFIX)
        && !node
            .record_path()
            .to_ascii_lowercase()
            .ends_with("_cargogrid_template")
}

fn fill(grid: &mut CargoGrid, container: &Container) {
    let field = |path: &str| Source {
        kind: SourceKind::Record,
        file: container.record_path.clone(),
        field: path.to_owned(),
    };
    match &container.shape {
        Shape::Open { dimensions } => {
            grid.shape = Some("open");
            grid.dimensions = Some(*dimensions);
            grid.scu = Some(round(volume(*dimensions) / CUBIC_METRES_PER_SCU));
            grid.cells = Some(cells(*dimensions));
            grid.source = Some(field("interiorDimensions"));
        }
        Shape::Closed { scu, field: path } => {
            grid.shape = Some("closed");
            grid.scu = Some(round(*scu));
            grid.source = Some(field(path));
        }
        Shape::Unknown(reason) => grid.issue = Some(reason.clone()),
    }
}

fn read_container(tree: &RecordTree<'_>) -> Container {
    let root = tree.root();
    let shape = match structure(Some(root), "inventoryType").map(StructValue::type_name) {
        Some("InventoryOpenContainerType") => structure(Some(root), "interiorDimensions")
            .map(|size| ["x", "y", "z"].map(|axis| number(Some(size), axis).map(f64::from)))
            .and_then(|[x, y, z]| Some([x?, y?, z?]))
            .map_or_else(
                || Shape::Unknown("the container states no interior dimensions".to_owned()),
                open,
            ),
        Some("InventoryClosedContainerType") => closed(root),
        Some(other) => Shape::Unknown(format!("the container type {other} states no capacity")),
        None => Shape::Unknown("the container has no inventory type".to_owned()),
    };
    Container {
        record_path: tree.record_path().to_owned(),
        shape,
    }
}

fn open(dimensions: [f64; 3]) -> Shape {
    if dimensions
        .iter()
        .all(|side| *side > 0.0 && *side <= MAX_SIDE_METRES)
    {
        Shape::Open { dimensions }
    } else {
        Shape::Unknown(format!(
            "the interior dimensions {dimensions:?} are not lengths between 0 and {MAX_SIDE_METRES} m"
        ))
    }
}

fn closed(root: &StructValue<'_>) -> Shape {
    let capacity = structure(structure(Some(root), "inventoryType"), "capacity");
    let unit = capacity.map(StructValue::type_name);
    let (name, field, divisor, path) = match unit {
        Some("SStandardCargoUnit") => (
            "SStandardCargoUnit",
            "standardCargoUnits",
            1.0,
            "inventoryType/capacity/SStandardCargoUnit/standardCargoUnits",
        ),
        Some("SCentiCargoUnit") => (
            "SCentiCargoUnit",
            "centiSCU",
            100.0,
            "inventoryType/capacity/SCentiCargoUnit/centiSCU",
        ),
        Some("SMicroCargoUnit") => (
            "SMicroCargoUnit",
            "microSCU",
            1_000_000.0,
            "inventoryType/capacity/SMicroCargoUnit/microSCU",
        ),
        _ => {
            return Shape::Unknown("the container states no capacity in cargo units".to_owned());
        }
    };
    match number(capacity, field).map(|value| f64::from(value) / divisor) {
        Some(scu) if (0.0..=MAX_CLOSED_SCU).contains(&scu) => Shape::Closed { scu, field: path },
        Some(scu) => Shape::Unknown(format!(
            "the capacity {scu} SCU of {name} is not between 0 and {MAX_CLOSED_SCU}"
        )),
        None => Shape::Unknown(format!("the capacity {name} states no {field}")),
    }
}

fn volume([x, y, z]: [f64; 3]) -> f64 {
    x * y * z
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a floored, non-negative, finite number of cells"
)]
fn cells(dimensions: [f64; 3]) -> u64 {
    dimensions
        .iter()
        .map(|side| ((side / CELL_METRES) + CELL_TOLERANCE).floor().max(0.0) as u64)
        .fold(1_u64, u64::saturating_mul)
}

fn round(value: f64) -> f64 {
    let rounded = (value * 1e4).round() / 1e4;
    if rounded.abs() < f64::EPSILON {
        0.0
    } else {
        rounded
    }
}

/// Rounds a sum to four decimals.
pub(super) fn rounded(value: f64) -> f64 {
    round(value)
}
