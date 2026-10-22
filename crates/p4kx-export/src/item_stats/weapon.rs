use p4kx_extraction::{DataForge, RecordTree, StructValue, TreeBuilder, Value};

use super::parts::damage;
use super::walk::owner_of;
use crate::contract_stats::{AmmoStats, FireAction, WeaponStats};
use crate::record_fields::{component, elements, integer, number, reference, structure, text};

const SEQUENCE_ENTRIES: &str = "sequenceEntries";
const AMMO_PARAMS: &str = "AmmoParams";
const ENTITY: &str = "EntityClassDefinition";

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

const CONTAINER: &str = "SAmmoContainerComponentParams";

/// Returns the tree of the Record `reference` links to when it has the struct `type_name` and can
/// be built.
fn linked<'f>(
    reference: Option<crate::guid_text::CatalogGuid>,
    type_name: &str,
    forge: &'f DataForge<'f>,
    trees: &mut TreeBuilder<'f>,
) -> Option<RecordTree<'f>> {
    let record = forge.record_by_guid(reference?.0)?;
    if record.type_name().ok()? != type_name {
        return None;
    }
    trees.build(&record).ok()
}

/// Returns the capacity and ammunition of the container of `tree`.
fn loaded<'f>(
    container: Option<&StructValue<'f>>,
    forge: &'f DataForge<'f>,
    trees: &mut TreeBuilder<'f>,
) -> (Option<i64>, Option<AmmoStats>) {
    let ammo = linked(
        reference(container, "ammoParamsRecord"),
        AMMO_PARAMS,
        forge,
        trees,
    )
    .map(|tree| ammo(&tree));
    (integer(container, "maxAmmoCount"), ammo)
}

/// Returns the weapon block of the item `tree`; `None` without a weapon component.
///
/// A ship weapon holds its ammo container itself; a personal weapon links the magazine entity
/// through `ammoContainerRecord`, followed one hop. A Record that is missing, has another struct
/// or cannot be built leaves the ammunition `None` and never fails the item.
pub(super) fn weapon<'f>(
    tree: &RecordTree<'f>,
    forge: &'f DataForge<'f>,
    trees: &mut TreeBuilder<'f>,
) -> Option<WeaponStats<'f>> {
    let weapon = component(tree, "SCItemWeaponComponentParams")?;
    let mut fire_actions = Vec::new();
    for action in elements(Some(weapon), "fireActions") {
        if let Value::Struct(action) = action {
            collect(action, &mut fire_actions);
        }
    }
    let (ammo_capacity, ammo) = if let Some(container) = component(tree, CONTAINER) {
        loaded(Some(container), forge, trees)
    } else {
        let magazine = linked(
            reference(Some(weapon), "ammoContainerRecord"),
            ENTITY,
            forge,
            trees,
        );
        loaded(
            magazine
                .as_ref()
                .and_then(|magazine| component(magazine, CONTAINER)),
            forge,
            trees,
        )
    };
    Some(WeaponStats {
        fire_actions,
        ammo_capacity,
        ammo,
    })
}
