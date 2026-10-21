use super::read::is_format_character;
use super::{ValueOrigin, VehicleImplementation, VehiclePart};

/// How many values of a sum came from each [`ValueOrigin`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OriginTally {
    base: usize,
    patch: usize,
    elem: usize,
}

impl OriginTally {
    const fn add(&mut self, origin: ValueOrigin) {
        let slot = match origin {
            ValueOrigin::Base => &mut self.base,
            ValueOrigin::Patch => &mut self.patch,
            ValueOrigin::Elem => &mut self.elem,
        };
        *slot = slot.saturating_add(1);
    }

    /// Returns how many values came from `origin`.
    #[must_use]
    pub const fn count(&self, origin: ValueOrigin) -> usize {
        match origin {
            ValueOrigin::Base => self.base,
            ValueOrigin::Patch => self.patch,
            ValueOrigin::Elem => self.elem,
        }
    }

    /// Returns how many values were counted.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.base
            .saturating_add(self.patch)
            .saturating_add(self.elem)
    }
}

/// A part that destroys or detaches at a share of its hit points.
#[derive(Debug, Clone, PartialEq)]
pub struct PartThreshold {
    part: String,
    health: f64,
    damage: f64,
    origin: ValueOrigin,
}

impl PartThreshold {
    /// Returns the part's name.
    #[must_use]
    pub fn part(&self) -> &str {
        &self.part
    }

    /// Returns the part's `damageMax`.
    #[must_use]
    pub const fn health(&self) -> f64 {
        self.health
    }

    /// Returns the damage at which the part destroys the vehicle or detaches.
    #[must_use]
    pub const fn damage(&self) -> f64 {
        self.damage
    }

    /// Returns where the part's `damageMax` comes from.
    #[must_use]
    pub const fn origin(&self) -> ValueOrigin {
        self.origin
    }
}

/// A number attribute of a part that holds text but no number, with its raw text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableValue {
    part: String,
    attribute: &'static str,
    raw: String,
}

impl UnreadableValue {
    /// Returns the part's name.
    #[must_use]
    pub fn part(&self) -> &str {
        &self.part
    }

    /// Returns the attribute's name, e.g. `mass`.
    #[must_use]
    pub const fn attribute(&self) -> &'static str {
        self.attribute
    }

    /// Returns the value as stored.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }
}

/// The sums over the parts of a vehicle implementation: mass, hull hit points and the thresholds
/// at which parts destroy the vehicle or detach (ADR-0061).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VehicleTotals {
    mass: f64,
    mass_parts: usize,
    mass_origins: OriginTally,
    health: f64,
    health_parts: usize,
    health_origins: OriginTally,
    destruction: Vec<PartThreshold>,
    detach: Vec<PartThreshold>,
    unreadable: Vec<UnreadableValue>,
    cleaned_mass: usize,
    cleaned_damage: usize,
}

impl VehicleTotals {
    /// Returns the sum of the `mass` of every part, in kilograms.
    #[must_use]
    pub const fn mass(&self) -> f64 {
        self.mass
    }

    /// Returns how many parts state a mass.
    #[must_use]
    pub const fn mass_parts(&self) -> usize {
        self.mass_parts
    }

    /// Returns where the masses summed came from.
    #[must_use]
    pub const fn mass_origins(&self) -> OriginTally {
        self.mass_origins
    }

    /// Returns the sum of the `damageMax` of the structural parts: every part with a positive
    /// `damageMax` except those holding an item port flagged `uneditable` or `invisible`.
    #[must_use]
    pub const fn health(&self) -> f64 {
        self.health
    }

    /// Returns how many parts the health sums.
    #[must_use]
    pub const fn health_parts(&self) -> usize {
        self.health_parts
    }

    /// Returns where the `damageMax` values summed came from.
    #[must_use]
    pub const fn health_origins(&self) -> OriginTally {
        self.health_origins
    }

    /// Returns the parts that destroy the vehicle when they take `damageRatioMin` of their
    /// `damageMax`, in part order.
    #[must_use]
    pub fn destruction(&self) -> &[PartThreshold] {
        &self.destruction
    }

    /// Returns the parts that detach when they take `detachRatio` of their `damageMax` and have
    /// no destroying behaviour, in part order.
    #[must_use]
    pub fn detach(&self) -> &[PartThreshold] {
        &self.detach
    }

    /// Returns the `mass`, `damageMax` and `detachRatio` values that hold text but no number;
    /// they are left out of the sums, never counted as 0.
    #[must_use]
    pub fn unreadable(&self) -> &[UnreadableValue] {
        &self.unreadable
    }

    /// Returns how many numbers read carried invisible format characters that were dropped.
    #[must_use]
    pub const fn cleaned(&self) -> usize {
        self.cleaned_mass.saturating_add(self.cleaned_damage)
    }

    /// Returns how many `mass` numbers needed their format characters dropped.
    #[must_use]
    pub const fn cleaned_mass(&self) -> usize {
        self.cleaned_mass
    }

    /// Returns how many `damageMax` and `detachRatio` numbers needed their format characters
    /// dropped.
    #[must_use]
    pub const fn cleaned_damage(&self) -> usize {
        self.cleaned_damage
    }
}

impl VehicleImplementation {
    /// Returns the sums over the parts: mass, hull hit points and the destruction and detach
    /// thresholds.
    #[must_use]
    pub fn totals(&self) -> VehicleTotals {
        let mut totals = VehicleTotals::default();
        for part in self.iter() {
            add_part(&mut totals, part);
        }
        totals
    }
}

fn add_part(totals: &mut VehicleTotals, part: &VehiclePart) {
    let mass = number_of(totals, part, "mass", part.mass);
    if let Some(mass) = mass {
        totals.mass += mass;
        totals.mass_parts = totals.mass_parts.saturating_add(1);
        totals.mass_origins.add(part.origin_of("mass"));
    }
    let detach = number_of(totals, part, "detachRatio", part.detach_ratio);
    let Some(health) = number_of(totals, part, "damageMax", part.damage_max).filter(|h| *h > 0.0)
    else {
        return;
    };
    let origin = part.origin_of("damageMax");
    if counts_as_hull(part) {
        totals.health += health;
        totals.health_parts = totals.health_parts.saturating_add(1);
        totals.health_origins.add(origin);
    }
    let threshold = |damage: f64| PartThreshold {
        part: part.name.clone(),
        health,
        damage,
        origin,
    };
    if let Some(ratio) = part.destroy_ratio {
        let damage = ratio * health;
        if damage > 0.0 {
            totals.destruction.push(threshold(damage));
        }
    } else if let Some(ratio) = detach.filter(|ratio| *ratio > 0.0) {
        totals.detach.push(threshold(ratio * health));
    }
}

fn counts_as_hull(part: &VehiclePart) -> bool {
    part.item_port
        .as_ref()
        .is_none_or(|port| !(port.has_flag("uneditable") || port.has_flag("invisible")))
}

/// Returns `parsed`, or records the attribute `name` of `part` as unreadable when it holds text
/// that is no number; counts a number that needed its format characters dropped.
fn number_of(
    totals: &mut VehicleTotals,
    part: &VehiclePart,
    name: &'static str,
    parsed: Option<f64>,
) -> Option<f64> {
    let raw = part.attribute(name).filter(|raw| !raw.trim().is_empty())?;
    match parsed {
        Some(_) => {
            if raw.chars().any(is_format_character) {
                let counter = if name == "mass" {
                    &mut totals.cleaned_mass
                } else {
                    &mut totals.cleaned_damage
                };
                *counter = counter.saturating_add(1);
            }
        }
        None => totals.unreadable.push(UnreadableValue {
            part: part.name.clone(),
            attribute: name,
            raw: raw.to_owned(),
        }),
    }
    parsed
}
