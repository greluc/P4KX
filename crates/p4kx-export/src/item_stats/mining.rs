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
        instability: value("laserInstability"),
        optimal_charge_window_size: value("optimalChargeWindowSizeModifier"),
        resistance: value("resistanceModifier"),
        shatter_damage: value("shatterdamageModifier"),
        cluster_factor: value("clusterFactorModifier"),
        optimal_charge_window_rate: value("optimalChargeWindowRateModifier"),
        catastrophic_charge_window_rate: value("catastrophicChargeWindowRateModifier"),
    })
}

fn optimal_rate(laser: Option<&StructValue<'_>>) -> Option<bool> {
    boolean(laser, "isOptimalRateGood")
}

fn filter_value(parent: Option<&StructValue<'_>>) -> Option<f32> {
    modifier(structure(parent, "filterParams"), "filterModifier")
}

/// Returns the mining laser block of the item `tree`; `None` without
/// `SEntityComponentMiningLaserParams`.
pub(super) fn laser(tree: &RecordTree<'_>) -> Option<MiningLaserStats> {
    let laser = component(tree, "SEntityComponentMiningLaserParams")?;
    let modifiers = structure(Some(laser), "miningLaserModifiers");
    Some(MiningLaserStats {
        is_optimal_rate_good: optimal_rate(modifiers),
        throttle_lerp_speed: number(Some(laser), "throttleLerpSpeed"),
        throttle_minimum: number(Some(laser), "throttleMinimum"),
        uses_power_throttle: boolean(Some(laser), "usesPowerThrottle"),
        global_params_guid: reference(Some(laser), "globalParams"),
        modifiers: values(modifiers),
        filter_modifier: filter_value(Some(laser)),
    })
}

fn effect(entry: &StructValue<'_>) -> MiningEffect {
    let laser = structure(Some(entry), "MiningLaserModifier");
    MiningEffect {
        lifetime: lifetime(entry),
        is_optimal_rate_good: optimal_rate(laser),
        modifiers: values(laser),
    }
}

fn fire_action_modifier(entry: &StructValue<'_>) -> FireActionModifier {
    let stats = structure(structure(Some(entry), "weaponModifier"), "weaponStats");
    FireActionModifier {
        fire_action_index: integer(Some(entry), "fireActionIndex"),
        lifetime: lifetime(entry),
        damage_multiplier: number(stats, "damageMultiplier"),
    }
}

/// Returns the mining modifier block of the item `tree`; `None` when its
/// `EntityComponentAttachableModifierParams` holds no mining modifier.
///
/// Of each mining modifier struct the first one counts.
pub(super) fn modifier_block<'f>(tree: &RecordTree<'f>) -> Option<MiningModifierStats<'f>> {
    let attachable = component(tree, "EntityComponentAttachableModifierParams")?;
    let mut laser = None;
    let mut rock = None;
    let mut filter = None;
    let mut fire_action_modifiers = Vec::new();
    for entry in structs(Some(attachable), "modifiers") {
        match entry.type_name() {
            LASER_MODIFIER => laser = laser.or_else(|| Some(effect(entry))),
            ROCK_MODIFIER => rock = rock.or_else(|| Some(effect(entry))),
            FILTER_MODIFIER => {
                filter = filter.or_else(|| {
                    Some(FilterEffect {
                        lifetime: lifetime(entry),
                        filter_modifier: filter_value(Some(entry)),
                    })
                });
            }
            WEAPON_MODIFIER => fire_action_modifiers.push(fire_action_modifier(entry)),
            _ => {}
        }
    }
    if laser.is_none() && rock.is_none() && filter.is_none() {
        return None;
    }
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
