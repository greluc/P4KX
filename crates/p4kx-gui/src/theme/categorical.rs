// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The categorical colour tokens of the P4KX design: departments, planets and item classes.
//!
//! They carry identity only, never status or action.

use egui::Color32;

/// `color-dept-raumueberlegenheit`, Raumüberlegenheit.
pub const DEPT_RAUMUEBERLEGENHEIT: Color32 = Color32::from_rgb(0x37, 0xBB, 0xC0);
/// `color-dept-forschung`, Forschung.
pub const DEPT_FORSCHUNG: Color32 = Color32::from_rgb(0x35, 0x5D, 0xDC);
/// `color-dept-sub-radar`, Sub-Radar.
pub const DEPT_SUB_RADAR: Color32 = Color32::from_rgb(0xA3, 0x00, 0x0A);
/// `color-dept-marinekorps`, Marinekorps.
pub const DEPT_MARINEKORPS: Color32 = Color32::from_rgb(0x7A, 0x5E, 0x96);
/// `color-dept-profit`, Profit.
pub const DEPT_PROFIT: Color32 = Color32::from_rgb(0x23, 0x9E, 0x33);
/// `color-dept-search-rescue`, Search and Rescue.
pub const DEPT_SEARCH_RESCUE: Color32 = Color32::from_rgb(0xFF, 0xD2, 0x3F);
/// `color-dept-combat`, legacy alias of [`DEPT_RAUMUEBERLEGENHEIT`].
pub const DEPT_COMBAT: Color32 = DEPT_RAUMUEBERLEGENHEIT;
/// `color-dept-research`, legacy alias of [`DEPT_FORSCHUNG`].
pub const DEPT_RESEARCH: Color32 = DEPT_FORSCHUNG;
/// `color-dept-marine`, legacy alias of [`DEPT_MARINEKORPS`].
pub const DEPT_MARINE: Color32 = DEPT_MARINEKORPS;

/// A planet's column-header fill and the stripe of its body cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetTint {
    /// `planet-<p>`: the header fill.
    pub fill: Color32,
    /// `planet-<p>-stripe`: the 2px left stripe of the body cells.
    pub stripe: Color32,
}

const fn planet(fill: u32, stripe: u32) -> PlanetTint {
    PlanetTint {
        fill: hex(fill),
        stripe: hex(stripe),
    }
}

const fn hex(value: u32) -> Color32 {
    let [_, r, g, b] = value.to_be_bytes();
    Color32::from_rgb(r, g, b)
}

/// `planet-hurston`.
pub const PLANET_HURSTON: PlanetTint = planet(0x003A_2519, 0x007A_4A2E);
/// `planet-crusader`.
pub const PLANET_CRUSADER: PlanetTint = planet(0x003A_2C26, 0x00C9_7A6A);
/// `planet-arccorp`.
pub const PLANET_ARCCORP: PlanetTint = planet(0x003A_2C1A, 0x00B5_722B);
/// `planet-microtech`.
pub const PLANET_MICROTECH: PlanetTint = planet(0x001F_2C3A, 0x005A_8FB8);
/// `planet-pyro-1` to `planet-pyro-6`.
pub const PLANET_PYRO: [PlanetTint; 6] = [
    planet(0x003A_3019, 0x00B8_9538),
    planet(0x002A_2018, 0x008A_6F4A),
    planet(0x003A_1F1A, 0x00B8_503A),
    planet(0x001A_2A2A, 0x004A_8A8A),
    planet(0x002A_1A2A, 0x008A_4A8A),
    planet(0x002A_2A18, 0x008A_8A3A),
];
/// `planet-terra`.
pub const PLANET_TERRA: PlanetTint = planet(0x001A_323A, 0x003A_8AA8);
/// `planet-delamar`.
pub const PLANET_DELAMAR: PlanetTint = planet(0x0026_2626, 0x007A_7A7A);
/// `planet-hash-0` to `planet-hash-11`, the buckets of a planet without its own tint.
pub const PLANET_HASH: [PlanetTint; 12] = [
    planet(0x003A_2020, 0x00A8_5050),
    planet(0x003A_2C20, 0x00A8_7850),
    planet(0x003A_3A20, 0x00A8_A850),
    planet(0x002C_3A20, 0x0078_A850),
    planet(0x0020_3A20, 0x0050_A850),
    planet(0x0020_3A2C, 0x0050_A878),
    planet(0x0020_3A3A, 0x0050_A8A8),
    planet(0x0020_2C3A, 0x0050_78A8),
    planet(0x0020_203A, 0x0050_50A8),
    planet(0x002C_203A, 0x0078_50A8),
    planet(0x003A_203A, 0x00A8_50A8),
    planet(0x003A_202C, 0x00A8_5078),
];

/// An item class's chip hue and its label tint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemClassTint {
    /// `item-class-<c>`: the chip border and, at 15 %, its fill.
    pub hue: Color32,
    /// `item-class-<c>-text`: the chip label.
    pub text: Color32,
}

const fn item_class(hue: u32, text: u32) -> ItemClassTint {
    ItemClassTint {
        hue: hex(hue),
        text: hex(text),
    }
}

/// `item-class-military`.
pub const ITEM_CLASS_MILITARY: ItemClassTint = item_class(0x008A_9455, 0x00C3_CA96);
/// `item-class-civilian`.
pub const ITEM_CLASS_CIVILIAN: ItemClassTint = item_class(0x005C_84A8, 0x00AC_C5DC);
/// `item-class-industrial`.
pub const ITEM_CLASS_INDUSTRIAL: ItemClassTint = item_class(0x00A8_643F, 0x00E2_B59C);
/// `item-class-stealth`.
pub const ITEM_CLASS_STEALTH: ItemClassTint = item_class(0x009C_6688, 0x00D9_B5CB);
/// `item-class-competition`.
pub const ITEM_CLASS_COMPETITION: ItemClassTint = item_class(0x0044_9C80, 0x0091_D2BC);
