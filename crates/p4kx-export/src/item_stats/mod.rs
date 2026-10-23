use p4kx_extraction::{DataForge, RecordTree, TreeBuilder};

use crate::contract_stats::ItemStats;
use crate::record_fields::{component, structure, text};

mod gear;
mod parts;
mod resources;
mod walk;
mod weapon;

const POWER_PLANT: &str = "PowerPlant";
const COOLER: &str = "Cooler";

/// Returns the stat blocks of the item `tree`, `None` when it has none of the blocks.
///
/// A block follows from the components of the item: the weapon, shield, quantum drive, missile,
/// suit armour, clothing, radar and attachable modifier components, the tractor beam action of
/// the weapon, and the type `PowerPlant` or `Cooler` of its `AttachDef`. The ammunition of a
/// weapon is read from the Records the item links to, with `trees`; one that cannot be read
/// leaves the ammunition `null` and never fails the item.
pub(crate) fn read<'f>(
    tree: &RecordTree<'f>,
    forge: &'f DataForge<'f>,
    trees: &mut TreeBuilder<'f>,
) -> Option<ItemStats<'f>> {
    let attach = structure(component(tree, "SAttachableComponentParams"), "AttachDef");
    let kind = text(attach, "Type");
    let is = |name: &str| kind.is_some_and(|kind| kind.eq_ignore_ascii_case(name));
    let weapon = weapon::weapon(tree, forge, trees);
    let shield = parts::shield(tree);
    let quantum_drive = parts::quantum_drive(tree);
    let resource_states = resources::states(tree);
    let power_plant = is(POWER_PLANT).then(|| resources::power_plant(&resource_states));
    let cooler = is(COOLER).then(|| resources::cooler(&resource_states));
    let missile = parts::missile(tree);
    let armor = gear::armor(tree, forge, trees);
    let clothing = gear::clothing(tree);
    let radar = gear::radar(tree);
    let tractor_beam = gear::tractor_beam(tree);
    let weapon_modifier = gear::weapon_modifier(tree);
    if weapon.is_none()
        && shield.is_none()
        && quantum_drive.is_none()
        && power_plant.is_none()
        && cooler.is_none()
        && missile.is_none()
        && armor.is_none()
        && clothing.is_none()
        && radar.is_none()
        && tractor_beam.is_none()
        && weapon_modifier.is_none()
    {
        return None;
    }
    Some(ItemStats {
        weapon,
        shield,
        quantum_drive,
        power_plant,
        cooler,
        missile,
        armor,
        clothing,
        radar,
        tractor_beam,
        weapon_modifier,
        health: parts::health(tree),
        resource_states,
    })
}
