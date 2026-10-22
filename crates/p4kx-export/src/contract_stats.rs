use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::every_field_required;

/// The typed stat blocks of an item, raw facts of the Archive (ADR-0065).
///
/// Every block is `null` for an item that does not have it; the item has `stats` only when it
/// has at least one of the six blocks `weapon`, `shield`, `quantumDrive`, `powerPlant`, `cooler`
/// and `missile`. No value is derived: a number is what the Archive states, in the unit the game
/// stores it.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct ItemStats<'a> {
    /// The weapon block, from `SCItemWeaponComponentParams`.
    pub(crate) weapon: Option<WeaponStats<'a>>,
    /// The shield generator block, from `SCItemShieldGeneratorParams`.
    pub(crate) shield: Option<ShieldStats>,
    /// The quantum drive block, from `SCItemQuantumDriveParams`.
    pub(crate) quantum_drive: Option<QuantumDriveStats>,
    /// The power plant block, for an item of the type `PowerPlant`.
    pub(crate) power_plant: Option<PowerPlantStats>,
    /// The cooler block, for an item of the type `Cooler`.
    pub(crate) cooler: Option<CoolerStats>,
    /// The missile block, from `SCItemMissileParams`.
    pub(crate) missile: Option<MissileStats<'a>>,
    /// The power connection, from `EntityComponentPowerConnection`.
    pub(crate) power: Option<PowerConnection>,
    /// The heat connection, from `EntityComponentHeatConnection`.
    pub(crate) heat: Option<HeatConnection>,
    /// The hit points, from `SHealthComponentParams.Health`.
    pub(crate) health: Option<f32>,
}

/// A weapon: its fire modes and the ammunition it fires.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct WeaponStats<'a> {
    /// The fire modes of `fireActions` in stored order; a sequence is flattened to its entries.
    pub(crate) fire_actions: Vec<FireAction<'a>>,
    /// The rounds the weapon holds, from `SAmmoContainerComponentParams.maxAmmoCount`.
    pub(crate) ammo_capacity: Option<i64>,
    /// The ammunition, from the `AmmoParams` Record that
    /// `SAmmoContainerComponentParams.ammoParamsRecord` links to; `null` without a readable one.
    pub(crate) ammo: Option<AmmoStats>,
}

/// One fire mode of a weapon.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct FireAction<'a> {
    /// The struct of the fire mode, e.g. `SWeaponActionFireRapidParams`. A closed set the game
    /// may extend; the schema keeps a string.
    pub(crate) kind: &'a str,
    /// The `name` of the fire mode.
    pub(crate) name: Option<&'a str>,
    /// `fireRate`, in rounds per minute.
    pub(crate) fire_rate: Option<f32>,
    /// `launchParams.pellets`, the projectiles of one shot.
    pub(crate) pellets: Option<i64>,
    /// `launchParams.ammoCost`, the rounds one shot uses.
    pub(crate) ammo_cost: Option<i64>,
    /// `heatPerShot`.
    pub(crate) heat_per_shot: Option<f32>,
    /// `chargeTime` of a charged fire mode, in seconds.
    pub(crate) charge_time: Option<f32>,
    /// `overchargeTime` of a charged fire mode, in seconds.
    pub(crate) overcharge_time: Option<f32>,
    /// `cooldownTime` of a charged fire mode, in seconds.
    pub(crate) cooldown_time: Option<f32>,
}

/// The ammunition of a weapon.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct AmmoStats {
    /// `speed`, in metres per second.
    pub(crate) speed: Option<f32>,
    /// `lifetime`, in seconds.
    pub(crate) lifetime: Option<f32>,
    /// The damage on impact, from `projectileParams.damage`.
    pub(crate) damage: Option<Damage>,
    /// The damage of the detonation, from the `damage` of `explosionParams`.
    pub(crate) detonation_damage: Option<Damage>,
    /// `explosionParams.minRadius`, in metres.
    pub(crate) detonation_min_radius: Option<f32>,
    /// `explosionParams.maxRadius`, in metres.
    pub(crate) detonation_max_radius: Option<f32>,
}

/// Damage by type, as the Archive states it.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct Damage {
    /// `DamagePhysical`.
    pub(crate) physical: Option<f32>,
    /// `DamageEnergy`.
    pub(crate) energy: Option<f32>,
    /// `DamageDistortion`.
    pub(crate) distortion: Option<f32>,
    /// `DamageThermal`.
    pub(crate) thermal: Option<f32>,
    /// `DamageBiochemical`.
    pub(crate) biochemical: Option<f32>,
    /// `DamageStun`.
    pub(crate) stun: Option<f32>,
}

/// A shield generator.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct ShieldStats {
    /// `MaxShieldHealth`.
    pub(crate) max_health: Option<f32>,
    /// `MaxShieldRegen`, in hit points per second.
    pub(crate) max_regen: Option<f32>,
    /// `DecayRatio`.
    pub(crate) decay_ratio: Option<f32>,
    /// `DownedRegenDelay`, in seconds.
    pub(crate) downed_regen_delay: Option<f32>,
    /// `DamagedRegenDelay`, in seconds.
    pub(crate) damaged_regen_delay: Option<f32>,
}

/// A quantum drive.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct QuantumDriveStats {
    /// `quantumFuelRequirement`, as stored.
    pub(crate) fuel_requirement: Option<f32>,
    /// `jumpRange`, in metres.
    pub(crate) jump_range: Option<f32>,
    /// `disconnectRange`, in metres.
    pub(crate) disconnect_range: Option<f32>,
    /// `params.driveSpeed`, in metres per second.
    pub(crate) drive_speed: Option<f32>,
    /// `params.cooldownTime`, in seconds.
    pub(crate) cooldown_time: Option<f32>,
    /// `params.spoolUpTime`, in seconds.
    pub(crate) spool_up_time: Option<f32>,
    /// `params.stageOneAccelRate`.
    pub(crate) stage_one_accel_rate: Option<f32>,
    /// `params.stageTwoAccelRate`.
    pub(crate) stage_two_accel_rate: Option<f32>,
    /// `params.engageSpeed`, in metres per second.
    pub(crate) engage_speed: Option<f32>,
}

/// A power plant.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct PowerPlantStats {
    /// The amount per second of the `ItemResourceDeltaGeneration` whose resource is `Power`.
    pub(crate) power_output: Option<f32>,
}

/// A cooler.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct CoolerStats {
    /// The amount per second of the `ItemResourceDeltaGeneration` whose resource is `Coolant`.
    pub(crate) cooling_rate: Option<f32>,
}

/// A missile.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct MissileStats<'a> {
    /// The damage of the explosion, from the `damage` of `explosionParams`.
    pub(crate) damage: Option<Damage>,
    /// `explosionParams.minRadius`, in metres.
    pub(crate) explosion_min_radius: Option<f32>,
    /// `explosionParams.maxRadius`, in metres.
    pub(crate) explosion_max_radius: Option<f32>,
    /// `targetingParams.trackingSignalType`. A closed set the game may extend; the schema keeps a
    /// string.
    pub(crate) tracking_signal_type: Option<&'a str>,
    /// `targetingParams.lockTime`, in seconds.
    pub(crate) lock_time: Option<f32>,
    /// `targetingParams.lockingAngle`, in degrees.
    pub(crate) locking_angle: Option<f32>,
    /// `targetingParams.lockRangeMin`, in metres.
    pub(crate) lock_range_min: Option<f32>,
    /// `targetingParams.lockRangeMax`, in metres.
    pub(crate) lock_range_max: Option<f32>,
    /// `gcsParams.linearSpeed`, in metres per second.
    pub(crate) linear_speed: Option<f32>,
}

/// The power connection of an item.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct PowerConnection {
    /// `PowerBase`.
    pub(crate) power_base: Option<f32>,
    /// `PowerDraw`.
    pub(crate) power_draw: Option<f32>,
    /// `PowerToEM`, the electromagnetic signature per power.
    pub(crate) power_to_em: Option<f32>,
    /// `DecayRateOfEM`.
    pub(crate) decay_rate_of_em: Option<f32>,
}

/// The heat connection of an item.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct HeatConnection {
    /// `ThermalEnergyBase`.
    pub(crate) thermal_energy_base: Option<f32>,
    /// `ThermalEnergyDraw`.
    pub(crate) thermal_energy_draw: Option<f32>,
    /// `TemperatureToIR`, the infrared signature per temperature.
    pub(crate) temperature_to_ir: Option<f32>,
    /// `OverheatTemperature`.
    pub(crate) overheat_temperature: Option<f32>,
}
