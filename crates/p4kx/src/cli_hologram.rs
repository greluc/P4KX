// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use clap::Args;
use p4kx_extraction::gfx::{MAX_CANVAS_PIXELS, MAX_CANVAS_SIDE};

use crate::cli_entity::LoadoutChangeArgs;
use crate::cli_model::lod_range;

/// Arguments of `p4kx ui hologram`.
#[derive(Debug, Args)]
pub(crate) struct HologramArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The class name, GUID or record path of the Entity, e.g. `AEGS_Avenger_Titan`.
    pub(crate) entity: String,
    /// The PNG file to write; replaced if it exists.
    #[arg(long, value_name = "FILE")]
    pub(crate) out: PathBuf,
    /// The picture size in pixels: `WIDTHxHEIGHT`, or `SIDE` alone for a square. At most 16384
    /// a side and 33554432 pixels, and no more than the graphics device draws.
    #[arg(long, value_name = "SIZE", default_value = "1280x720", value_parser = parse_size)]
    pub(crate) size: [u32; 2],
    /// The colour under the hologram: `transparent`, or `#RRGGBB` or `#RRGGBBAA`.
    #[arg(long, value_name = "COLOUR", default_value = "transparent", value_parser = parse_background)]
    pub(crate) background: [u8; 4],
    /// The angle in degrees between the nose and the direction the camera looks from, positive
    /// towards the right of the vehicle. Defaults to `ownDefaultViewAngle` of the
    /// `UIHoloVehicle_Config` Record of the Archive, 60 without one.
    #[arg(long, value_name = "DEGREES", value_parser = parse_angle, allow_negative_numbers = true)]
    pub(crate) view_angle: Option<f32>,
    /// The height of the camera above the horizontal plane as an angle in degrees, -80 to 80.
    #[arg(long, value_name = "DEGREES", default_value_t = 20.0, value_parser = parse_elevation, allow_negative_numbers = true)]
    pub(crate) elevation: f32,
    /// The vertical field of view in degrees, 1 to 120. Defaults to `cameraFOV` of the
    /// `UIHoloVehicle_Config` Record, 10 without one.
    #[arg(long, value_name = "DEGREES", value_parser = parse_fov)]
    pub(crate) fov: Option<f32>,
    /// The factor on the distance at which the vehicle just fits the picture, 0.5 to 10.
    /// Defaults to `ownCameraDistanceScaler` of the `UIHoloVehicle_Config` Record, 1.5 without
    /// one.
    #[arg(long, value_name = "FACTOR", value_parser = parse_scaler)]
    pub(crate) distance_scaler: Option<f32>,
    /// The colour of the hologram, `#RRGGBB`.
    #[arg(long, value_name = "COLOUR", default_value = "#4DC7FF", value_parser = parse_tint)]
    pub(crate) tint: [u8; 3],
    /// How opaque the body of the hologram is, 0 to 1; the edges are more opaque.
    #[arg(long, value_name = "OPACITY", default_value_t = 0.22, value_parser = parse_opacity)]
    pub(crate) opacity: f32,
    /// How strongly the edges glow, 0 to 4.
    #[arg(long, value_name = "STRENGTH", default_value_t = 1.0, value_parser = parse_glow)]
    pub(crate) glow: f32,
    /// The level of detail of the models; a model without it is drawn at the nearest lower level.
    #[arg(long, value_name = "N", default_value_t = 0, value_parser = lod_range())]
    pub(crate) lod: u32,
    /// Draws on the software adapter (WARP, lavapipe) instead of the graphics card, so the
    /// picture does not depend on the graphics card.
    #[arg(long)]
    pub(crate) software: bool,
    /// Changes of the Loadout.
    #[command(flatten)]
    pub(crate) changes: LoadoutChangeArgs,
}

fn parse_size(text: &str) -> Result<[u32; 2], String> {
    let side = |text: &str| {
        text.trim()
            .parse::<u32>()
            .ok()
            .filter(|&side| (1..=MAX_CANVAS_SIDE).contains(&side))
            .ok_or_else(|| format!("`{text}` is no size from 1 to {MAX_CANVAS_SIDE} pixels"))
    };
    let (width, height) = match text.split_once(['x', 'X']) {
        Some((width, height)) => (side(width)?, side(height)?),
        None => (side(text)?, side(text)?),
    };
    if u64::from(width) * u64::from(height) > MAX_CANVAS_PIXELS {
        return Err(format!(
            "{width} x {height} is more than {MAX_CANVAS_PIXELS} pixels"
        ));
    }
    Ok([width, height])
}

fn hex(text: &str, digits: &[usize]) -> Option<Vec<u8>> {
    let hex = text.strip_prefix('#')?;
    if !digits.contains(&hex.len()) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    (0..hex.len() / 2)
        .map(|at| u8::from_str_radix(hex.get(at * 2..at * 2 + 2)?, 16).ok())
        .collect()
}

fn parse_background(text: &str) -> Result<[u8; 4], String> {
    if text == "transparent" {
        return Ok([0; 4]);
    }
    match hex(text, &[6, 8]).as_deref() {
        Some(&[red, green, blue]) => Ok([red, green, blue, u8::MAX]),
        Some(&[red, green, blue, alpha]) => Ok([red, green, blue, alpha]),
        _ => Err(format!(
            "`{text}` is no colour: use transparent, #RRGGBB or #RRGGBBAA"
        )),
    }
}

fn parse_tint(text: &str) -> Result<[u8; 3], String> {
    match hex(text, &[6]).as_deref() {
        Some(&[red, green, blue]) => Ok([red, green, blue]),
        _ => Err(format!("`{text}` is no colour: use #RRGGBB")),
    }
}

fn ranged(text: &str, what: &str, range: std::ops::RangeInclusive<f32>) -> Result<f32, String> {
    text.trim()
        .parse::<f32>()
        .ok()
        .filter(|value| range.contains(value))
        .ok_or_else(|| {
            format!(
                "`{text}` is no {what} from {} to {}",
                range.start(),
                range.end()
            )
        })
}

fn parse_angle(text: &str) -> Result<f32, String> {
    ranged(text, "angle in degrees", -360.0..=360.0)
}

fn parse_elevation(text: &str) -> Result<f32, String> {
    ranged(text, "elevation in degrees", -80.0..=80.0)
}

fn parse_fov(text: &str) -> Result<f32, String> {
    ranged(text, "field of view in degrees", 1.0..=120.0)
}

fn parse_scaler(text: &str) -> Result<f32, String> {
    ranged(text, "factor", 0.5..=10.0)
}

fn parse_opacity(text: &str) -> Result<f32, String> {
    ranged(text, "opacity", 0.0..=1.0)
}

fn parse_glow(text: &str) -> Result<f32, String> {
    ranged(text, "strength", 0.0..=4.0)
}
