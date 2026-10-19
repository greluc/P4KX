use p4kx_extraction::RecordTree;

use crate::contract::{Item, Manufacturer, Ship};
use crate::item::item;
use crate::item_class::ClassLine;
use crate::manufacturer::manufacturer;
use crate::ship::ship;
use crate::texts::Texts;
use crate::{ItemClassSummary, Kind, KindSummary};

/// A Record a kind selected: its entity, or a development leftover of the kind.
pub(crate) enum Picked<'f> {
    /// A manufacturer.
    Manufacturer(Manufacturer<'f>),
    /// An item and the `Class:` line of its English description.
    Item(Item<'f>, ClassLine),
    /// A ship.
    Ship(Ship<'f>),
    /// A development leftover of the kind.
    Skipped(Kind),
}

impl<'f> Picked<'f> {
    /// Returns the entity of `kind` the Record tree `tree` describes, its texts resolved by
    /// `texts`.
    pub(crate) fn entity(kind: Kind, tree: &RecordTree<'f>, texts: &Texts<'f>) -> Self {
        match kind {
            Kind::Manufacturers => Self::Manufacturer(manufacturer(tree, texts)),
            Kind::Items => {
                let (item, line) = item(tree, texts);
                Self::Item(item, line)
            }
            Kind::Ships => Self::Ship(ship(tree, texts)),
        }
    }
}

/// The entities of every kind, ordered by GUID, with the counts of the summary.
#[derive(Default)]
pub(crate) struct Collected<'f> {
    pub(crate) manufacturers: Vec<Manufacturer<'f>>,
    pub(crate) items: Vec<Item<'f>>,
    pub(crate) ships: Vec<Ship<'f>>,
    pub(crate) item_classes: ItemClassSummary,
    skipped: [usize; Kind::ALL.len()],
}

impl<'f> Collected<'f> {
    /// Sorts `picked` into the kinds and orders every kind by the text of the GUID.
    pub(crate) fn new(picked: Vec<Picked<'f>>) -> Self {
        let mut collected = Self::default();
        for pick in picked {
            match pick {
                Picked::Manufacturer(entity) => collected.manufacturers.push(entity),
                Picked::Item(entity, line) => {
                    collected.item_classes.count(line);
                    collected.items.push(entity);
                }
                Picked::Ship(entity) => collected.ships.push(entity),
                Picked::Skipped(kind) => {
                    if let Some(skipped) = collected.skipped.get_mut(kind.index()) {
                        *skipped = skipped.saturating_add(1);
                    }
                }
            }
        }
        collected
            .manufacturers
            .sort_by_cached_key(|entity| entity.guid.text());
        collected
            .items
            .sort_by_cached_key(|entity| entity.guid.text());
        collected
            .ships
            .sort_by_cached_key(|entity| entity.guid.text());
        collected
    }

    /// Returns the counts of every kind, in the order of [`Kind::ALL`].
    pub(crate) fn summary(&self, complete: bool) -> Vec<KindSummary> {
        Kind::ALL
            .into_iter()
            .map(|kind| KindSummary {
                kind,
                count: match kind {
                    Kind::Manufacturers => self.manufacturers.len(),
                    Kind::Items => self.items.len(),
                    Kind::Ships => self.ships.len(),
                },
                skipped: self.skipped.get(kind.index()).copied().unwrap_or_default(),
                complete,
            })
            .collect()
    }
}
