//! The vehicle implementation of a ship: the XML file its `VehicleComponentParams` name, with the
//! Modification the Entity selects applied.

mod locate;
mod patch;
mod read;

use std::collections::HashMap;

use crate::merge::port_key;
use crate::{LoadoutError, LoadoutFiles};

pub(crate) use locate::load;

/// Whether a part of the vehicle implementation shows its item port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PortVisibility {
    /// The vehicle implementation names no part of the port.
    #[default]
    Unspecified,
    /// The part's item port is shown.
    Visible,
    /// The part's item port carries the flag `invisible`.
    Invisible,
}

impl PortVisibility {
    /// Returns the stable code of the visibility, e.g. `invisible`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unspecified => "unspecified",
            Self::Visible => "visible",
            Self::Invisible => "invisible",
        }
    }
}

/// What a part of the vehicle implementation is, by its `class`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartKind {
    /// A part that holds an item port, `class="ItemPort"`.
    ItemPort,
    /// A tread of a tracked vehicle, `class="Tread"`.
    Tread,
    /// A wheel, `class="SubPartWheel"`.
    Wheel,
    /// Any other part, e.g. `Animated`, `AnimatedJoint` or `Static`.
    Other,
}

/// The item port of a part of the vehicle implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VehicleItemPort {
    pub(crate) min_size: Option<u32>,
    pub(crate) max_size: Option<u32>,
    pub(crate) flags: String,
}

impl VehicleItemPort {
    /// Returns the smallest item size the port takes, or `None` when the XML gives none.
    #[must_use]
    pub const fn min_size(&self) -> Option<u32> {
        self.min_size
    }

    /// Returns the largest item size the port takes, or `None` when the XML gives none.
    #[must_use]
    pub const fn max_size(&self) -> Option<u32> {
        self.max_size
    }

    /// Returns the flags as stored, e.g. `invisible $uneditable`.
    #[must_use]
    pub fn flags(&self) -> &str {
        &self.flags
    }

    /// Returns whether the flags contain `flag`, ignoring ASCII case and a leading `$`.
    #[must_use]
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags
            .split_ascii_whitespace()
            .any(|token| token.trim_start_matches('$').eq_ignore_ascii_case(flag))
    }

    /// Returns [`PortVisibility::Invisible`] when the port carries the flag `invisible`.
    #[must_use]
    pub fn visibility(&self) -> PortVisibility {
        if self.has_flag("invisible") {
            PortVisibility::Invisible
        } else {
            PortVisibility::Visible
        }
    }
}

/// One part of the vehicle implementation: the hull, a hardpoint, a tread or a wheel.
#[derive(Debug, Clone, PartialEq)]
pub struct VehiclePart {
    pub(crate) name: String,
    pub(crate) class: String,
    pub(crate) kind: PartKind,
    pub(crate) mass: Option<f64>,
    pub(crate) damage_max: Option<f64>,
    pub(crate) detach_ratio: Option<f64>,
    pub(crate) item_port: Option<VehicleItemPort>,
    pub(crate) tread_wheels: Vec<String>,
    pub(crate) children: Vec<Self>,
}

impl VehiclePart {
    /// Returns the part name; an item port carries it as its port name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the `class` as stored, e.g. `ItemPort` or `Animated`.
    #[must_use]
    pub fn class(&self) -> &str {
        &self.class
    }

    /// Returns what the part is.
    #[must_use]
    pub const fn kind(&self) -> PartKind {
        self.kind
    }

    /// Returns the part's mass in kilograms, or `None` when it gives none or an unreadable one.
    #[must_use]
    pub const fn mass(&self) -> Option<f64> {
        self.mass
    }

    /// Returns the damage the part takes before it is destroyed, or `None` when it gives none.
    #[must_use]
    pub const fn damage_max(&self) -> Option<f64> {
        self.damage_max
    }

    /// Returns the part of its damage at which it detaches, or `None` when it gives none.
    #[must_use]
    pub const fn detach_ratio(&self) -> Option<f64> {
        self.detach_ratio
    }

    /// Returns the part's item port, or `None` when it holds none.
    #[must_use]
    pub const fn item_port(&self) -> Option<&VehicleItemPort> {
        self.item_port.as_ref()
    }

    /// Returns the part names of the wheels a tread runs on; empty for any other part.
    #[must_use]
    pub fn tread_wheels(&self) -> &[String] {
        &self.tread_wheels
    }

    /// Returns the parts below this one, in file order.
    #[must_use]
    pub fn children(&self) -> &[Self] {
        &self.children
    }
}

/// One landing gear of an Entity's `VehicleLandingGearSystem` component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandingGear {
    pub(crate) bone: String,
    pub(crate) geometry: String,
}

impl LandingGear {
    /// Returns the name of the bone the gear moves.
    #[must_use]
    pub fn bone(&self) -> &str {
        &self.bone
    }

    /// Returns the path of the gear's geometry; empty when the component gives none.
    #[must_use]
    pub fn geometry(&self) -> &str {
        &self.geometry
    }
}

/// The vehicle implementation of an Entity: its parts after the Modification is applied.
#[derive(Debug, Clone, PartialEq)]
pub struct VehicleImplementation {
    pub(crate) path: String,
    pub(crate) patch_path: Option<String>,
    pub(crate) modification: Option<String>,
    pub(crate) name: String,
    pub(crate) parts: Vec<VehiclePart>,
    pub(crate) visibility: HashMap<String, PortVisibility>,
    pub(crate) issues: Vec<LoadoutError>,
}

impl VehicleImplementation {
    /// Reads the vehicle implementation of the Entity `class_name` from `files`, as its
    /// `VehicleComponentParams` name it: `definition` is the `vehicleDefinition` and
    /// `modification` the Modification to apply, if the Entity selects one.
    ///
    /// The file is `definition`; when `files` hold none of that path, the file of the same name
    /// in the implementation folder, then the file named after `class_name`. The patch file of
    /// the Modification is its `patchFile`, else `Modifications/<definition name>_<modification>`,
    /// else `Modifications/<class name>`. A patch file or Modification that cannot be found or
    /// read is an [issue](Self::issues); the implementation without it is still returned.
    ///
    /// # Errors
    ///
    /// Returns [`LoadoutError::VehicleImplementation`] when no file holds the implementation or
    /// it is no readable `Vehicle` document.
    pub fn read(
        files: &dyn LoadoutFiles,
        class_name: &str,
        definition: &str,
        modification: Option<&str>,
    ) -> Result<Self, LoadoutError> {
        load(
            files,
            class_name,
            definition,
            modification.unwrap_or_default().trim(),
        )
    }

    /// Returns the path of the implementation file as the Archive spells it, or as the Record
    /// names it when the Archive gives another.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the path of the Modification's patch file that was applied, if one was.
    #[must_use]
    pub fn patch_path(&self) -> Option<&str> {
        self.patch_path.as_deref()
    }

    /// Returns the name of the Modification the Entity selects, if it selects one.
    #[must_use]
    pub fn modification(&self) -> Option<&str> {
        self.modification.as_deref()
    }

    /// Returns the `name` of the `Vehicle` element.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the top-level parts, in file order; parts marked `skipPart` are left out.
    #[must_use]
    pub fn parts(&self) -> &[VehiclePart] {
        &self.parts
    }

    /// Returns why the implementation is not read in full, e.g. a patch file that is missing.
    #[must_use]
    pub fn issues(&self) -> &[LoadoutError] {
        &self.issues
    }

    /// Returns every part, depth first in file order.
    pub fn iter(&self) -> impl Iterator<Item = &VehiclePart> {
        let mut stack: Vec<&VehiclePart> = self.parts.iter().rev().collect();
        std::iter::from_fn(move || {
            let part = stack.pop()?;
            stack.extend(part.children.iter().rev());
            Some(part)
        })
    }

    /// Returns the parts holding an item port.
    pub fn item_ports(&self) -> impl Iterator<Item = &VehiclePart> {
        self.iter().filter(|part| part.item_port.is_some())
    }

    /// Returns the treads.
    pub fn treads(&self) -> impl Iterator<Item = &VehiclePart> {
        self.iter().filter(|part| part.kind == PartKind::Tread)
    }

    /// Returns the wheels, the parts of class `SubPartWheel`.
    pub fn wheels(&self) -> impl Iterator<Item = &VehiclePart> {
        self.iter().filter(|part| part.kind == PartKind::Wheel)
    }

    /// Returns the visibility of the port `name`, matching the part names ignoring ASCII case;
    /// the first part of that name decides.
    #[must_use]
    pub fn port_visibility(&self, name: &str) -> PortVisibility {
        self.visibility
            .get(&port_key(name))
            .copied()
            .unwrap_or_default()
    }
}
