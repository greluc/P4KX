use std::fmt::Write as _;
use std::io::{self, Write};

use p4kx_assembly::{EntityNode, LandingGear, PortVisibility, VehicleImplementation, VehiclePart};
use serde::Serialize;

use crate::query_table::one_line;

/// The JSON shape of the vehicle implementation of an Entity.
#[derive(Serialize)]
pub(crate) struct VehicleJson<'a> {
    path: &'a str,
    patch_path: Option<&'a str>,
    modification: Option<&'a str>,
    name: &'a str,
    ports: Vec<VehiclePortJson<'a>>,
    treads: Vec<TreadJson<'a>>,
    wheels: Vec<&'a str>,
}

#[derive(Serialize)]
struct VehiclePortJson<'a> {
    name: &'a str,
    visibility: &'static str,
    min_size: Option<u32>,
    max_size: Option<u32>,
    flags: &'a str,
}

#[derive(Serialize)]
struct TreadJson<'a> {
    name: &'a str,
    wheels: &'a [String],
}

impl<'a> From<&'a VehicleImplementation> for VehicleJson<'a> {
    fn from(vehicle: &'a VehicleImplementation) -> Self {
        Self {
            path: vehicle.path(),
            patch_path: vehicle.patch_path(),
            modification: vehicle.modification(),
            name: vehicle.name(),
            ports: vehicle.item_ports().filter_map(port_json).collect(),
            treads: vehicle
                .treads()
                .map(|tread| TreadJson {
                    name: tread.name(),
                    wheels: tread.tread_wheels(),
                })
                .collect(),
            wheels: vehicle.wheels().map(VehiclePart::name).collect(),
        }
    }
}

fn port_json(part: &VehiclePart) -> Option<VehiclePortJson<'_>> {
    let item = part.item_port()?;
    Some(VehiclePortJson {
        name: part.name(),
        visibility: item.visibility().code(),
        min_size: item.min_size(),
        max_size: item.max_size(),
        flags: item.flags(),
    })
}

/// The JSON shape of a landing gear.
#[derive(Serialize)]
pub(crate) struct LandingGearJson<'a> {
    bone: &'a str,
    geometry: &'a str,
    mass: Option<f64>,
}

impl<'a> From<&'a LandingGear> for LandingGearJson<'a> {
    fn from(gear: &'a LandingGear) -> Self {
        Self {
            bone: gear.bone(),
            geometry: gear.geometry(),
            mass: gear.mass(),
        }
    }
}

/// Returns the code of a visibility the vehicle implementation states, `None` for none.
pub(crate) const fn visibility_code(visibility: PortVisibility) -> Option<&'static str> {
    match visibility {
        PortVisibility::Unspecified => None,
        PortVisibility::Visible | PortVisibility::Invisible => Some(visibility.code()),
    }
}

/// Prints the vehicle implementation and the landing gear of `node` below its header line.
pub(crate) fn print(out: &mut impl Write, node: &EntityNode, indent: &str) -> io::Result<()> {
    if let Some(vehicle) = node.vehicle() {
        let mut line = format!("{indent}vehicle: {}", vehicle.path());
        if let Some(modification) = vehicle.modification() {
            let _ = write!(line, " (modification {modification}");
            if let Some(patch) = vehicle.patch_path() {
                let _ = write!(line, ", patch {patch}");
            }
            line.push(')');
        }
        writeln!(out, "{}", one_line(&line))?;
        for tread in vehicle.treads() {
            writeln!(
                out,
                "{}",
                one_line(&format!(
                    "{indent}tread: {} (wheels {})",
                    tread.name(),
                    tread.tread_wheels().join(", ")
                ))
            )?;
        }
        let wheels: Vec<&str> = vehicle.wheels().map(VehiclePart::name).collect();
        if !wheels.is_empty() {
            writeln!(
                out,
                "{}",
                one_line(&format!("{indent}wheels: {}", wheels.join(", ")))
            )?;
        }
    }
    if !node.landing_gear().is_empty() {
        let bones: Vec<&str> = node.landing_gear().iter().map(LandingGear::bone).collect();
        writeln!(
            out,
            "{}",
            one_line(&format!("{indent}landing gear: {}", bones.join(", ")))
        )?;
    }
    Ok(())
}
