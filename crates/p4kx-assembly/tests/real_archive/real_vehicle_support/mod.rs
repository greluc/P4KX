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
