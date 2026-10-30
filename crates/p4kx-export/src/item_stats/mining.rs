// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{RecordTree, StructValue};

use crate::contract_stats_mining::{
    FilterEffect, FireActionModifier, MiningEffect, MiningLaserStats, MiningModifierStats,
    MiningModifierValues,
};
use crate::record_fields::{
    boolean, component, integer, number, reference, structs, structure, text,
};

const LASER_MODIFIER: &str = "ItemMiningModifierParams";
const ROCK_MODIFIER: &str = "ItemMineableRockModifierParams";
const FILTER_MODIFIER: &str = "MiningFilterItemModifierParams";
const WEAPON_MODIFIER: &str = "ItemWeaponModifiersParams";

/// Returns the `value` of the `FloatModifierMultiplicative` the field `name` of `parent` holds.
fn modifier(parent: Option<&StructValue<'_>>, name: &str) -> Option<f32> {
    number(structure(parent, name), "value")
}

fn lifetime(parent: &StructValue<'_>) -> Option<f32> {
    number(structure(Some(parent), "modifierLifetime"), "lifetime")
}

fn values(laser: Option<&StructValue<'_>>) -> Option<MiningModifierValues> {
    let laser = laser?;
    let value = |name| modifier(Some(laser), name);
    Some(MiningModifierValues {
        is_optimal_rate_good: boolean(Some(laser), "isOptimalRateGood"),
        instability: value("laserInstability"),
        optimal_charge_window_size: value("optimalChargeWindowSizeModifier"),
        resistance: value("resistanceModifier"),
        shatter_damage: value("shatterdamageModifier"),
        cluster_factor: value("clusterFactorModifier"),
        optimal_charge_window_rate: value("optimalChargeWindowRateModifier"),
        catastrophic_charge_window_rate: value("catastrophicChargeWindowRateModifier"),
    })
}

fn filter_value(parent: Option<&StructValue<'_>>) -> Option<f32> {
    modifier(structure(parent, "filterParams"), "filterModifier")
}

/// Returns the mining laser block of the item `tree`; `None` without
/// `SEntityComponentMiningLaserParams`.
pub(super) fn laser(tree: &RecordTree<'_>) -> Option<MiningLaserStats> {
    let laser = component(tree, "SEntityComponentMiningLaserParams")?;
    Some(MiningLaserStats {
        throttle_lerp_speed: number(Some(laser), "throttleLerpSpeed"),
        throttle_minimum: number(Some(laser), "throttleMinimum"),
        uses_power_throttle: boolean(Some(laser), "usesPowerThrottle"),
        global_params_guid: reference(Some(laser), "globalParams"),
        modifiers: values(structure(Some(laser), "miningLaserModifiers")),
        filter_modifier: filter_value(Some(laser)),
    })
}

/// Returns the mining modifier block of the item `tree`; `None` when its
/// `EntityComponentAttachableModifierParams` holds no mining modifier.
pub(super) fn modifier_block<'f>(tree: &RecordTree<'f>) -> Option<MiningModifierStats<'f>> {
    let attachable = component(tree, "EntityComponentAttachableModifierParams")?;
    let first = |type_name: &str| {
        structs(Some(attachable), "modifiers").find(|entry| entry.type_name() == type_name)
    };
    let effect = |entry: &StructValue<'_>| MiningEffect {
        lifetime: lifetime(entry),
        modifiers: values(structure(Some(entry), "MiningLaserModifier")),
    };
    let laser = first(LASER_MODIFIER).map(effect);
    let rock = first(ROCK_MODIFIER).map(effect);
    let filter = first(FILTER_MODIFIER).map(|entry| FilterEffect {
        lifetime: lifetime(entry),
        filter_modifier: filter_value(Some(entry)),
    });
    if laser.is_none() && rock.is_none() && filter.is_none() {
        return None;
    }
    let fire_action_modifiers = structs(Some(attachable), "modifiers")
        .filter(|entry| entry.type_name() == WEAPON_MODIFIER)
        .map(|entry| FireActionModifier {
            fire_action_index: integer(Some(entry), "fireActionIndex"),
            lifetime: lifetime(entry),
            damage_multiplier: number(
                structure(structure(Some(entry), "weaponModifier"), "weaponStats"),
                "damageMultiplier",
            ),
        })
        .collect();
    Some(MiningModifierStats {
        activation_method: text(Some(attachable), "activationMethod"),
        charges: integer(Some(attachable), "charges"),
        can_interrupt: boolean(Some(attachable), "canInterrupt"),
        is_interruptible: boolean(Some(attachable), "isInterruptible"),
        laser,
        rock,
        filter,
        fire_action_modifiers,
    })
}
