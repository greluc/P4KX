use std::collections::{HashMap, HashSet};

use p4kx_extraction::{Archive, DataForge, TagTree};
use rayon::prelude::*;

use crate::PaintSummary;
use crate::contract::{Item, Paint, Ship};
use crate::guid_text::CatalogGuid;
use crate::paint_port::{PaintPort, VehicleFiles, VehiclePorts};

const PAINT_TYPE: &str = "Paints";

/// The paints of a catalog and what was counted deriving them.
pub(crate) struct Paints<'f> {
    pub(crate) paints: Vec<Paint<'f>>,
    pub(crate) summary: PaintSummary,
}

/// Returns the paints among `items` with the ships of `ships` they fit.
///
/// A paint is an item whose `AttachDef.Type` is `Paints`. It fits a ship when the paint port of
/// a port of the ship that takes `Paints` accepts it: every tag in the port's required tags is
/// among the paint's `Tags` or above one of them in the tag tree. The ports are the
/// `SItemPortDef` entries of the ship's Record and the `ItemPort` elements of its vehicle
/// implementation.
pub(crate) fn paints<'f>(
    archive: &Archive,
    forge: &DataForge<'_>,
    items: &[Item<'f>],
    ships: &[Ship<'f>],
) -> Paints<'f> {
    let candidates: Vec<&Item<'f>> = items
        .iter()
        .filter(|item| {
            item.r#type
                .is_some_and(|kind| kind.eq_ignore_ascii_case(PAINT_TYPE))
        })
        .collect();
    if candidates.is_empty() {
        return Paints {
            paints: Vec::new(),
            summary: PaintSummary::default(),
        };
    }
    let vehicles = vehicles(archive, ships);
    let (by_ports, unreadable) = group_ships(&vehicles, ships);
    let tags = TagTree::build(forge);
    let index = TagIndex::new(&tags);
    let mut paints: Vec<Paint<'f>> = candidates
        .par_iter()
        .map(|item| {
            let have = lowered(&item.tags);
            let mut ship_guids: Vec<CatalogGuid> = by_ports
                .iter()
                .filter(|(ports, _)| ports.iter().any(|port| fits(port, &have, &index)))
                .flat_map(|(_, guids)| guids.iter().copied())
                .collect();
            ship_guids.sort_unstable_by_key(|guid| guid.text());
            Paint {
                guid: item.guid,
                class_name: item.class_name,
                record_path: item.record_path,
                name: item.name.clone(),
                ship_guids,
            }
        })
        .collect();
    paints.sort_by_cached_key(|paint| paint.guid.text());
    let summary = summarise(&paints, unreadable);
    Paints { paints, summary }
}

type ShipsByPorts = HashMap<Vec<PaintPort>, Vec<CatalogGuid>>;

/// Groups the ships by their paint ports, those of their Record and those of their vehicle
/// implementation, and counts the ships whose files could not be used.
fn group_ships<'v>(
    vehicles: &'v HashMap<Key<'v>, VehiclePorts>,
    ships: &[Ship<'v>],
) -> (ShipsByPorts, usize) {
    let mut by_ports = ShipsByPorts::new();
    let mut unreadable = 0_usize;
    for ship in ships {
        let mut ports = ship.paint_ports.clone();
        if let Some(found) = vehicle_of(vehicles, ship) {
            unreadable = unreadable.saturating_add(usize::from(found.unreadable));
            ports.extend(found.ports.iter().cloned());
        }
        ports.sort_unstable();
        ports.dedup();
        if !ports.is_empty() {
            by_ports.entry(ports).or_default().push(ship.guid);
        }
    }
    (by_ports, unreadable)
}

fn summarise(paints: &[Paint<'_>], unreadable_vehicles: usize) -> PaintSummary {
    let with_paints: HashSet<[u8; 36]> = paints
        .iter()
        .flat_map(|paint| paint.ship_guids.iter().map(|guid| guid.text()))
        .collect();
    let assigned = paints
        .iter()
        .filter(|paint| !paint.ship_guids.is_empty())
        .count();
    PaintSummary {
        paints: paints.len(),
        assigned,
        unassigned: paints.len().saturating_sub(assigned),
        ships: with_paints.len(),
        unreadable_vehicles,
    }
}

type Key<'s> = (&'s str, &'s str);

fn key<'s>(ship: &Ship<'s>) -> Option<Key<'s>> {
    Some((
        ship.vehicle_definition?,
        ship.modification.unwrap_or_default(),
    ))
}

fn vehicle_of<'v>(
    vehicles: &'v HashMap<Key<'v>, VehiclePorts>,
    ship: &Ship<'v>,
) -> Option<&'v VehiclePorts> {
    vehicles.get(&key(ship)?)
}

/// Reads the paint ports of every distinct vehicle implementation and Modification of `ships`.
fn vehicles<'s>(archive: &Archive, ships: &[Ship<'s>]) -> HashMap<Key<'s>, VehiclePorts> {
    let wanted: HashSet<Key<'s>> = ships.iter().filter_map(key).collect();
    let files = VehicleFiles::new(archive);
    wanted
        .into_par_iter()
        .map(|(definition, modification)| {
            let modification = (!modification.is_empty()).then_some(modification);
            (
                (definition, modification.unwrap_or_default()),
                files.paint_ports(definition, modification),
            )
        })
        .collect()
}

fn lowered(tags: &[&str]) -> Vec<String> {
    tags.iter().map(|tag| tag.to_ascii_lowercase()).collect()
}

fn fits(port: &PaintPort, have: &[String], index: &TagIndex<'_>) -> bool {
    !port.required_tags.is_empty()
        && port
            .required_tags
            .iter()
            .all(|wanted| have.iter().any(|tag| index.matches(tag, wanted)))
}

/// The tags of the tag tree by lower-case name.
struct TagIndex<'t> {
    tree: &'t TagTree,
    positions: HashMap<String, Vec<usize>>,
}

impl<'t> TagIndex<'t> {
    fn new(tree: &'t TagTree) -> Self {
        let mut positions: HashMap<String, Vec<usize>> = HashMap::new();
        for tag in tree.iter() {
            positions
                .entry(tag.name().to_ascii_lowercase())
                .or_default()
                .push(tag.position());
        }
        Self { tree, positions }
    }

    /// Returns whether the tag `have` is the tag `wanted` or lies below a tag of that name.
    fn matches(&self, have: &str, wanted: &str) -> bool {
        if have == wanted {
            return true;
        }
        let (Some(below), Some(above)) = (self.positions.get(have), self.positions.get(wanted))
        else {
            return false;
        };
        below.iter().filter_map(|&at| self.tree.at(at)).any(|tag| {
            above
                .iter()
                .filter_map(|&at| self.tree.at(at))
                .any(|ancestor| tag.is_within(&ancestor))
        })
    }
}
