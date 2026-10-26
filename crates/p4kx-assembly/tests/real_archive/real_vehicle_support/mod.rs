// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the real-Archive vehicle test.

use p4kx_assembly::{
    ArchiveFiles, Attachment, EntityNode, LoadoutFiles, PortVisibility, VehicleImplementation,
    VehiclePart,
};

pub(crate) fn nodes(root: &EntityNode) -> Vec<&EntityNode> {
    let mut all = vec![root];
    all.extend(
        root.walk()
            .into_iter()
            .filter_map(|(_, port)| match port.attachment() {
                Attachment::Entity(child) => Some(child.as_ref()),
                _ => None,
            }),
    );
    all
}

pub(crate) fn depth(parts: &[VehiclePart]) -> usize {
    parts
        .iter()
        .map(|part| depth(part.children()).saturating_add(1))
        .max()
        .unwrap_or(0)
}

pub(crate) fn mass(vehicle: &VehicleImplementation) -> f64 {
    vehicle.iter().filter_map(VehiclePart::mass).sum()
}

pub(crate) fn port_names(vehicle: &VehicleImplementation) -> Vec<&str> {
    vehicle.item_ports().map(VehiclePart::name).collect()
}

pub(crate) fn file_len(files: &ArchiveFiles<'_>, path: &str) -> usize {
    files.read(path).map_or(0, |bytes| bytes.len())
}

/// What the item ports and applied Elems of the vehicle implementations give.
#[derive(Debug, Default)]
pub(crate) struct PortFacts {
    named: usize,
    typed: usize,
    subtyped: usize,
    elems: usize,
}

impl PortFacts {
    pub(crate) fn note(&mut self, vehicle: &VehicleImplementation) {
        for port in vehicle.item_ports().filter_map(VehiclePart::item_port) {
            self.named += usize::from(port.display_name().is_some());
            self.typed += usize::from(!port.item_types().is_empty());
            self.subtyped += usize::from(
                port.item_types()
                    .iter()
                    .any(|kind| !kind.subtypes().is_empty()),
            );
        }
        self.elems += vehicle.elems().len();
    }

    pub(crate) fn summary(&self) -> String {
        format!(
            "item ports with a display name {}, with types {}, with sub-types {}; Elems applied {}",
            self.named, self.typed, self.subtyped, self.elems
        )
    }

    pub(crate) fn assert_found(&self) {
        assert!(
            self.typed > 0 && self.subtyped > 0,
            "no item port gives types with sub-types: {}/{}",
            self.typed,
            self.subtyped
        );
        assert!(self.elems > 0, "no Elem was applied");
        assert!(self.named > 0, "no item port has a display name");
    }
}

/// Returns the item ports whose visibility the Modification flips between visible and invisible.
pub(crate) fn flipped_ports<'a>(
    base: &'a VehicleImplementation,
    patched: &VehicleImplementation,
) -> Vec<&'a str> {
    base.item_ports()
        .map(VehiclePart::name)
        .filter(|name| {
            matches!(
                (base.port_visibility(name), patched.port_visibility(name)),
                (PortVisibility::Visible, PortVisibility::Invisible)
                    | (PortVisibility::Invisible, PortVisibility::Visible)
            )
        })
        .collect()
}
