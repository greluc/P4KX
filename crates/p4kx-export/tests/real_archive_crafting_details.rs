//! The crafting data of a real Archive through [`read_crafting`]; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.

use std::error::Error;

use p4kx_export::{BlueprintTiers, Cost, Crafting, ModifierKind, read_crafting};
use p4kx_extraction::{Archive, DataForge, DataForgeEntry, Guid};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const MIN_BLUEPRINTS: usize = 1_000;
const MIN_POOLS: usize = 100;
const MIN_SOURCES: usize = 1_000;
const CREATURE_DEFAULT: &str = "1dcde125-1275-474f-b0d7-d6682e9e521f";
const PYRO: &str = "286cb603-b4ae-4279-80a1-d4505fee1916";
const LMG_MAGAZINE: &str = "BP_CRAFT_behr_lmg_ballistic_01_mag";
const COVALEX_POOL: &str = "907989eb-9ee8-46a8-b794-032da6bd8a5d";
const COVALEX_BLUEPRINT: &str = "33be7f1a-3f75-4627-bb8b-88e6d6b42f4e";
const MULTIPLIER_BLUEPRINT: &str = "BP_CRAFT_slaver_armor_medium_arms_01_01_01";
const EXPECTED_MULTIPLIER: f32 = 1.15;

fn guid(text: &str) -> Result<Guid, Box<dyn Error>> {
    Ok(text.parse()?)
}

fn selections<'c, 'f>(cost: &'c Cost<'f>, out: &mut Vec<&'c p4kx_export::Selection<'f>>) {
    if let Cost::Select(selection) = cost {
        out.push(selection);
        for option in &selection.options {
            selections(option, out);
        }
    }
}

fn costs<'c, 'f>(blueprint: &'c BlueprintTiers<'f>) -> Vec<&'c Cost<'f>> {
    blueprint
        .tiers
        .iter()
        .flat_map(|tier| tier.mandatory.iter().chain(&tier.optional))
        .collect()
}

fn leaves<'c, 'f>(cost: &'c Cost<'f>, out: &mut Vec<&'c Cost<'f>>) {
    match cost {
        Cost::Select(selection) => {
            for option in &selection.options {
                leaves(option, out);
            }
        }
        other => out.push(other),
    }
}

fn multipliers<'f>(crafting: &Crafting<'f>) -> Vec<(&'f str, f32)> {
    let mut found = Vec::new();
    for blueprint in &crafting.blueprints {
        let mut all = Vec::new();
        for cost in costs(blueprint) {
            leaves(cost, &mut all);
        }
        for leaf in all {
            let multiplier = match leaf {
                Cost::Resource(line) => line.context.quantity_multiplier,
                Cost::Item(line) => line.context.quantity_multiplier,
                _ => None,
            };
            if let Some(multiplier) = multiplier {
                found.push((blueprint.class_name, multiplier));
            }
        }
    }
    found
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn reads_the_crafting_data_of_a_real_archive() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entry = DataForgeEntry::read(&archive)?;
    verify(&entry.open()?)
}

pub(crate) fn verify(forge: &DataForge<'_>) -> TestResult {
    let crafting = read_crafting(forge, None, None)?;
    assert!(crafting.failures.is_empty(), "{:?}", crafting.failures);
    assert!(crafting.blueprints.len() >= MIN_BLUEPRINTS, "blueprints");
    assert!(
        crafting.blueprints.iter().all(|bp| !bp.tiers.is_empty()),
        "every blueprint has a tier"
    );
    assert!(
        crafting.blueprints.is_sorted_by_key(|bp| bp.guid),
        "ordered by GUID"
    );
    check_selections(&crafting)?;
    check_modifiers(&crafting);
    check_distributions(&crafting)?;
    check_rewards(&crafting)?;
    check_dismantle(&crafting)
}

fn check_selections(crafting: &Crafting<'_>) -> TestResult {
    let magazine = crafting
        .blueprints
        .iter()
        .find(|bp| bp.class_name == LMG_MAGAZINE)
        .ok_or("the LMG magazine blueprint")?;
    let tier = magazine.tiers.first().ok_or("a tier")?;
    assert_eq!(tier.craft_time_seconds, Some(10), "craft time");
    let Some(Cost::Select(outer)) = &tier.mandatory else {
        return Err("a selection as mandatory cost".into());
    };
    assert_eq!(outer.count, Some(2), "choose 2 of 2");
    assert_eq!(outer.options.len(), 2, "two slots");
    let fewer = crafting
        .blueprints
        .iter()
        .filter(|bp| {
            let mut found = Vec::new();
            for cost in costs(bp) {
                selections(cost, &mut found);
            }
            found.iter().any(|selection| selection.chooses_fewer())
        })
        .count();
    assert_eq!(fewer, 3, "blueprints that let the player choose fewer");
    let found = multipliers(crafting);
    assert_eq!(found.len(), 3, "three lines carry a multiplier: {found:?}");
    assert!(
        found
            .iter()
            .all(|(_, multiplier)| (multiplier - EXPECTED_MULTIPLIER).abs() < f32::EPSILON),
        "factor 1.15"
    );
    assert!(
        found.iter().any(|(name, _)| *name == MULTIPLIER_BLUEPRINT),
        "the slaver arms"
    );
    Ok(())
}

fn check_modifiers(crafting: &Crafting<'_>) {
    let mut ranges = Vec::new();
    for blueprint in &crafting.blueprints {
        for cost in costs(blueprint) {
            let mut found = Vec::new();
            selections(cost, &mut found);
            for selection in found {
                for modifier in &selection.context.modifiers {
                    ranges.extend(modifier.ranges.iter().copied());
                }
            }
        }
    }
    assert!(
        ranges
            .iter()
            .any(|range| range.kind == ModifierKind::Multiplier
                && range.start_quality == Some(0)
                && range.end_quality == Some(1000)
                && range.at_start.is_some()
                && range.at_end.is_some()),
        "a linear multiplier over the whole quality range"
    );
    assert!(
        ranges
            .iter()
            .any(|range| range.kind == ModifierKind::Additive && range.at_start.is_some()),
        "an additive range"
    );
    assert!(!crafting.properties.is_empty(), "property definitions");
}

fn check_distributions(crafting: &Crafting<'_>) -> TestResult {
    let default = crafting
        .quality_distributions
        .iter()
        .find(|item| item.guid == guid(CREATURE_DEFAULT).unwrap_or_default())
        .ok_or("the default creature distribution")?;
    let curve = default.distribution.as_ref().ok_or("a curve")?;
    assert_eq!(curve.kind, "CraftingQualityDistributionNormal", "kind");
    assert_eq!(
        (curve.min, curve.max, curve.mean, curve.stddev),
        (Some(1.0), Some(1000.0), Some(269.0), Some(254.0)),
        "the curve"
    );
    let pyro = guid(PYRO)?;
    assert!(
        crafting
            .quality_distributions
            .iter()
            .flat_map(|item| &item.overrides)
            .any(|over| over.location == Some(pyro) && over.distribution.is_some()),
        "an override for Pyro"
    );
    Ok(())
}

fn check_rewards(crafting: &Crafting<'_>) -> TestResult {
    assert!(crafting.pools.len() >= MIN_POOLS, "pools");
    assert!(crafting.reward_sources.len() >= MIN_SOURCES, "sources");
    let pool = crafting
        .pools
        .iter()
        .find(|pool| pool.guid == guid(COVALEX_POOL).unwrap_or_default())
        .ok_or("the Covalex pool")?;
    assert_eq!(pool.entries.len(), 9, "nine entries");
    let chances = crafting.reward_chances();
    let blueprint = guid(COVALEX_BLUEPRINT)?;
    let from_pool: Vec<f64> = chances
        .get(&blueprint)
        .ok_or("chances for the blueprint")?
        .iter()
        .filter(|chance| chance.pool == pool.guid)
        .map(|chance| chance.chance)
        .collect();
    assert!(!from_pool.is_empty(), "contracts reward it");
    assert!(
        from_pool
            .iter()
            .all(|chance| (chance - 1.0 / 9.0).abs() < 1.0e-6),
        "chance 1 times weight 1 over 9: {from_pool:?}"
    );
    Ok(())
}

fn check_dismantle(crafting: &Crafting<'_>) -> TestResult {
    let process = crafting.dismantle.first().ok_or("a dismantle process")?;
    assert_eq!(process.efficiency, Some(0.5), "efficiency");
    assert_eq!(process.time_seconds, Some(15), "time");
    let magazine = crafting
        .blueprints
        .iter()
        .find(|bp| bp.class_name == LMG_MAGAZINE)
        .ok_or("the magazine blueprint")?;
    let link = crafting
        .dismantle_return(magazine)
        .ok_or("a dismantle return for the magazine")?;
    assert_eq!(link.process.guid, process.guid, "the process");
    assert_eq!(Some(link.item), magazine.produced_item, "the item");
    assert_eq!(link.blueprint, magazine.guid, "the blueprint");
    assert!(
        crafting.global.default_blueprints.contains(&process.guid),
        "the dismantle process is a default blueprint"
    );
    assert!(
        !crafting.global.dismantle_blacklist_resources.is_empty(),
        "blacklisted resources"
    );
    Ok(())
}
