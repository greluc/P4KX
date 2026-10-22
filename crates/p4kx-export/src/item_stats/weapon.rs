use p4kx_extraction::{DataForge, DataForgeError, RecordTree, StructValue, TreeBuilder, Value};

use super::parts::damage;
use super::walk::owner_of;
use crate::contract_stats::{AmmoStats, FireAction, WeaponStats};
use crate::record_fields::{component, elements, integer, number, reference, structure, text};

const SEQUENCE_ENTRIES: &str = "sequenceEntries";

fn collect<'f>(action: &StructValue<'f>, actions: &mut Vec<FireAction<'f>>) {
    if action.field(SEQUENCE_ENTRIES).is_some() {
        for entry in elements(Some(action), SEQUENCE_ENTRIES) {
            if let Value::Struct(entry) = entry
                && let Some(inner) = structure(Some(entry), "weaponAction")
            {
                collect(inner, actions);
            }
        }
        return;
    }
    let inner = structure(Some(action), "weaponAction");
    let launch =
        structure(Some(action), "launchParams").or_else(|| structure(inner, "launchParams"));
    let value = |name| number(Some(action), name).or_else(|| number(inner, name));
    actions.push(FireAction {
        kind: action.type_name(),
        name: text(Some(action), "name").or_else(|| text(inner, "name")),
        fire_rate: value("fireRate"),
        pellets: integer(launch, "pellets"),
        ammo_cost: integer(launch, "ammoCost"),
        heat_per_shot: value("heatPerShot"),
        charge_time: value("chargeTime"),
        overcharge_time: value("overchargeTime"),
        cooldown_time: value("cooldownTime"),
    });
}

fn ammo(tree: &RecordTree<'_>) -> AmmoStats {
    let projectile = structure(Some(tree.root()), "projectileParams");
    let explosion = owner_of(tree.root(), "explosionParams")
        .and_then(|owner| structure(Some(owner), "explosionParams"));
    AmmoStats {
        speed: number(Some(tree.root()), "speed"),
        lifetime: number(Some(tree.root()), "lifetime"),
        damage: damage(structure(projectile, "damage")),
        detonation_damage: damage(explosion),
        detonation_min_radius: number(explosion, "minRadius"),
        detonation_max_radius: number(explosion, "maxRadius"),
    }
}

/// Returns the weapon block of the item `tree`, the ammunition read from the Record the ammo
/// container links to; `None` without a weapon component.
///
/// # Errors
///
/// Returns the error of building the ammunition Record.
pub(super) fn weapon<'f>(
    tree: &RecordTree<'f>,
    forge: &'f DataForge<'f>,
    trees: &mut TreeBuilder<'f>,
) -> Result<Option<WeaponStats<'f>>, DataForgeError> {
    let Some(weapon) = component(tree, "SCItemWeaponComponentParams") else {
        return Ok(None);
    };
    let mut fire_actions = Vec::new();
    for action in elements(Some(weapon), "fireActions") {
        if let Value::Struct(action) = action {
            collect(action, &mut fire_actions);
        }
    }
    let container = component(tree, "SAmmoContainerComponentParams");
    let loaded = match reference(container, "ammoParamsRecord")
        .and_then(|guid| forge.record_by_guid(guid.0))
    {
        Some(record) => Some(ammo(&trees.build(&record)?)),
        None => None,
    };
    Ok(Some(WeaponStats {
        fire_actions,
        ammo_capacity: integer(container, "maxAmmoCount"),
        ammo: loaded,
    }))
}
