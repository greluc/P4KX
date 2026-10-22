use p4kx_extraction::{RecordTree, StructValue};

use super::walk::owner_of;
use crate::contract_stats::{
    Damage, HeatConnection, MissileStats, PowerConnection, QuantumDriveStats, ShieldStats,
};
use crate::record_fields::{component, number, structure, text};

const DAMAGE_FIELDS: [&str; 6] = [
    "DamagePhysical",
    "DamageEnergy",
    "DamageDistortion",
    "DamageThermal",
    "DamageBiochemical",
    "DamageStun",
];

/// Returns the damage the first struct below `root` that states a damage type holds.
pub(super) fn damage(root: Option<&StructValue<'_>>) -> Option<Damage> {
    let root = root?;
    let owner = DAMAGE_FIELDS.iter().find_map(|name| owner_of(root, name))?;
    let value = |name| number(Some(owner), name);
    Some(Damage {
        physical: value("DamagePhysical"),
        energy: value("DamageEnergy"),
        distortion: value("DamageDistortion"),
        thermal: value("DamageThermal"),
        biochemical: value("DamageBiochemical"),
        stun: value("DamageStun"),
    })
}

pub(super) fn shield(tree: &RecordTree<'_>) -> Option<ShieldStats> {
    let shield = component(tree, "SCItemShieldGeneratorParams");
    shield?;
    let value = |name| number(shield, name);
    Some(ShieldStats {
        max_health: value("MaxShieldHealth"),
        max_regen: value("MaxShieldRegen"),
        decay_ratio: value("DecayRatio"),
        downed_regen_delay: value("DownedRegenDelay"),
        damaged_regen_delay: value("DamagedRegenDelay"),
    })
}

pub(super) fn quantum_drive(tree: &RecordTree<'_>) -> Option<QuantumDriveStats> {
    let drive = component(tree, "SCItemQuantumDriveParams");
    drive?;
    let params = structure(drive, "params");
    let value = |name| number(params, name);
    Some(QuantumDriveStats {
        fuel_requirement: number(drive, "quantumFuelRequirement"),
        jump_range: number(drive, "jumpRange"),
        disconnect_range: number(drive, "disconnectRange"),
        drive_speed: value("driveSpeed"),
        cooldown_time: value("cooldownTime"),
        spool_up_time: value("spoolUpTime"),
        stage_one_accel_rate: value("stageOneAccelRate"),
        stage_two_accel_rate: value("stageTwoAccelRate"),
        engage_speed: value("engageSpeed"),
    })
}

pub(super) fn missile<'f>(tree: &RecordTree<'f>) -> Option<MissileStats<'f>> {
    let missile = component(tree, "SCItemMissileParams")?;
    let explosion = owner_of(missile, "explosionParams")
        .and_then(|owner| structure(Some(owner), "explosionParams"));
    let targeting = structure(Some(missile), "targetingParams");
    let guidance = structure(Some(missile), "GCSParams");
    Some(MissileStats {
        damage: damage(explosion),
        explosion_min_radius: number(explosion, "minRadius"),
        explosion_max_radius: number(explosion, "maxRadius"),
        tracking_signal_type: text(targeting, "trackingSignalType"),
        lock_time: number(targeting, "lockTime"),
        locking_angle: number(targeting, "lockingAngle"),
        lock_range_min: number(targeting, "lockRangeMin"),
        lock_range_max: number(targeting, "lockRangeMax"),
        linear_speed: number(guidance, "linearSpeed"),
    })
}

pub(super) fn power_connection(tree: &RecordTree<'_>) -> Option<PowerConnection> {
    let power = component(tree, "EntityComponentPowerConnection");
    power?;
    let value = |name| number(power, name);
    Some(PowerConnection {
        power_base: value("PowerBase"),
        power_draw: value("PowerDraw"),
        power_to_em: value("PowerToEM"),
        decay_rate_of_em: value("DecayRateOfEM"),
    })
}

pub(super) fn heat_connection(tree: &RecordTree<'_>) -> Option<HeatConnection> {
    let heat = component(tree, "EntityComponentHeatConnection");
    heat?;
    let value = |name| number(heat, name);
    Some(HeatConnection {
        thermal_energy_base: value("ThermalEnergyBase"),
        thermal_energy_draw: value("ThermalEnergyDraw"),
        temperature_to_ir: value("TemperatureToIR"),
        overheat_temperature: value("OverheatTemperature"),
    })
}

pub(super) fn health(tree: &RecordTree<'_>) -> Option<f32> {
    number(component(tree, "SHealthComponentParams"), "Health")
}
