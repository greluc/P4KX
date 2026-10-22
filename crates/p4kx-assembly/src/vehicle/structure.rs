use sha2::{Digest, Sha256};

use super::VehicleImplementation;
use crate::markup::Element;

const COSMETIC_ATTRIBUTES: [&str; 4] = ["material", "materials", "filename", "geometry"];
const MODIFICATIONS: &str = "Modifications";
const VEHICLE_NAME: &str = "name";

impl VehicleImplementation {
    /// Returns a 128-bit digest of the whole `Vehicle` element after the Modification is applied:
    /// every element below the root except `Modifications`, with all its attributes sorted by
    /// name, and the children in file order.
    ///
    /// Not compared are the file path, the names of the Modification and its patch file, the
    /// `name` of the root, the issues, and the attributes `material`, `materials`, `filename`
    /// and `geometry` anywhere, so that ships differing only in paint or model file digest
    /// equally.
    #[must_use]
    pub const fn structure_digest(&self) -> [u8; 16] {
        self.structure
    }
}

/// Returns the digest of the patched `Vehicle` element `root`.
pub(super) fn digest(root: &Element) -> [u8; 16] {
    let mut sha = Sha256::new();
    write_element(&mut sha, root, true);
    let full = sha.finalize();
    let mut out = [0_u8; 16];
    for (slot, byte) in out.iter_mut().zip(full.iter()) {
        *slot = *byte;
    }
    out
}

fn write_count(sha: &mut Sha256, count: usize) {
    sha.update((count as u64).to_le_bytes());
}

fn write_text(sha: &mut Sha256, text: &str) {
    write_count(sha, text.len());
    sha.update(text.as_bytes());
}

fn is_compared(name: &str, root: bool) -> bool {
    if root && name.eq_ignore_ascii_case(VEHICLE_NAME) {
        return false;
    }
    !COSMETIC_ATTRIBUTES
        .iter()
        .any(|cosmetic| name.eq_ignore_ascii_case(cosmetic))
}

fn write_element(sha: &mut Sha256, element: &Element, root: bool) {
    write_text(sha, &element.name.to_ascii_lowercase());
    let mut attributes: Vec<(String, &str)> = element
        .attributes
        .iter()
        .filter(|(name, _)| is_compared(name, root))
        .map(|(name, value)| (name.to_ascii_lowercase(), value.as_str()))
        .collect();
    attributes.sort_unstable();
    write_count(sha, attributes.len());
    for (name, value) in &attributes {
        write_text(sha, name);
        write_text(sha, value);
    }
    let children: Vec<&Element> = element
        .children
        .iter()
        .filter(|child| !child.name.eq_ignore_ascii_case(MODIFICATIONS))
        .collect();
    write_count(sha, children.len());
    for child in children {
        write_element(sha, child, false);
    }
}
