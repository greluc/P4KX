use p4kx_extraction::{DataForge, DataForgeError, RecordTree, TreeBuilder};

use crate::contract_stats::ItemStats;
use crate::record_fields::{component, structure, text};

mod parts;
mod walk;
mod weapon;

const POWER_PLANT: &str = "PowerPlant";
const COOLER: &str = "Cooler";

/// Returns the stat blocks of the item `tree`, `None` when it has none of the six blocks.
///
/// A block follows from the components of the item: the weapon, shield, quantum drive and missile
/// components, and the type `PowerPlant` or `Cooler` of its `AttachDef`. The ammunition of a
/// weapon is read from the Record the item links to, with `trees`.
///
/// # Errors
///
/// Returns the error of building a linked Record.
pub(crate) fn read<'f>(
    tree: &RecordTree<'f>,
    forge: &'f DataForge<'f>,
    trees: &mut TreeBuilder<'f>,
) -> Result<Option<ItemStats<'f>>, DataForgeError> {
    let attach = structure(component(tree, "SAttachableComponentParams"), "AttachDef");
    let kind = text(attach, "Type");
    let is = |name: &str| kind.is_some_and(|kind| kind.eq_ignore_ascii_case(name));
    let weapon = weapon::weapon(tree, forge, trees)?;
    let shield = parts::shield(tree);
    let quantum_drive = parts::quantum_drive(tree);
    let power_plant = is(POWER_PLANT).then(|| parts::power_plant(tree));
    let cooler = is(COOLER).then(|| parts::cooler(tree));
    let missile = parts::missile(tree);
    if weapon.is_none()
        && shield.is_none()
        && quantum_drive.is_none()
        && power_plant.is_none()
        && cooler.is_none()
        && missile.is_none()
    {
        return Ok(None);
    }
    Ok(Some(ItemStats {
        weapon,
        shield,
        quantum_drive,
        power_plant,
        cooler,
        missile,
        power: parts::power_connection(tree),
        heat: parts::heat_connection(tree),
        health: parts::health(tree),
    }))
}
