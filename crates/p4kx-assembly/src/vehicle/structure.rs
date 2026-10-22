use std::hash::{Hash, Hasher};

use super::{VehicleImplementation, VehicleItemPort, VehiclePart};

const COSMETIC_ATTRIBUTES: [&str; 4] = ["material", "materials", "filename", "geometry"];

impl VehicleImplementation {
    /// Feeds the structure of the implementation into `state`: every part with its attributes,
    /// numbers, item port and children, in file order, after the Modification is applied.
    ///
    /// The file path, the Modification and patch file names, the vehicle name and the issues are
    /// left out, as are the attributes `material`, `materials`, `filename` and `geometry`, so
    /// that two ships differing only in paint or model file feed the same bytes.
    pub fn hash_structure<H: Hasher>(&self, state: &mut H) {
        self.parts.len().hash(state);
        for part in &self.parts {
            part.hash_structure(state);
        }
    }
}

impl VehiclePart {
    fn hash_structure<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.class.hash(state);
        self.kind.hash(state);
        for number in [
            self.mass,
            self.damage_max,
            self.detach_ratio,
            self.destroy_ratio,
        ] {
            number.map(f64::to_bits).hash(state);
        }
        for (key, value) in &self.attributes {
            if COSMETIC_ATTRIBUTES
                .iter()
                .any(|cosmetic| key.eq_ignore_ascii_case(cosmetic))
            {
                continue;
            }
            key.hash(state);
            value.hash(state);
        }
        self.item_port.is_some().hash(state);
        if let Some(port) = &self.item_port {
            port.hash_structure(state);
        }
        self.tread_wheels.hash(state);
        self.children.len().hash(state);
        for child in &self.children {
            child.hash_structure(state);
        }
    }
}

impl VehicleItemPort {
    fn hash_structure<H: Hasher>(&self, state: &mut H) {
        self.min_size.hash(state);
        self.max_size.hash(state);
        self.flags.hash(state);
        self.id.hash(state);
        self.port_tags.hash(state);
        self.required_tags.hash(state);
        self.types.hash(state);
    }
}
