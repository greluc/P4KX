use std::hash::Hasher;
use std::io;

use p4kx_extraction::{RecordTree, Value};

use crate::contract_stats::ItemStats;
use crate::digest::{Digest, Wide, hash_value};
use crate::guid_text::CatalogGuid;
use crate::record_fields::{component, integer, reference, structure, text};

const COSMETIC_COMPONENTS: [&str; 8] = [
    "attachable",
    "purchasable",
    "geometry",
    "material",
    "display",
    "interact",
    "carryable",
    "usable",
];
const COSMETIC_FIELDS: [&str; 7] = [
    "Tags",
    "RequiredTags",
    "Skin",
    "Tint",
    "Color",
    "Colour",
    "Palette",
];
const COSMETIC_FIELD_PART: &str = "interaction";

fn is_cosmetic_field(name: &str) -> bool {
    COSMETIC_FIELDS
        .iter()
        .any(|cosmetic| name.eq_ignore_ascii_case(cosmetic))
        || name.to_ascii_lowercase().contains(COSMETIC_FIELD_PART)
}

/// What the items of one variant group share (`docs/basetool-catalog.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ItemKey<'f> {
    kind: &'f str,
    sub_type: Option<&'f str>,
    size: Option<i64>,
    grade: Option<i64>,
    manufacturer: Option<CatalogGuid>,
    components: Digest,
    stats: Digest,
}

/// Returns the key of the item `tree` with the stat blocks `stats`; `None` for an item without
/// an `AttachDef` type or without a component besides the cosmetic ones.
pub(crate) fn key<'f>(tree: &RecordTree<'f>, stats: Option<&ItemStats<'_>>) -> Option<ItemKey<'f>> {
    let definition = structure(component(tree, "SAttachableComponentParams"), "AttachDef");
    let kind = text(definition, "Type")?;
    let mut hasher = Wide::new();
    let mut any_compared = false;
    if let Some(Value::Array(components)) = tree.field("Components") {
        for entry in components {
            if let Value::Struct(inner) = entry
                && !is_cosmetic_component(inner.type_name())
            {
                any_compared = true;
                hash_value(&mut hasher, entry, &is_cosmetic_field);
            }
        }
    }
    if !any_compared {
        return None;
    }
    Some(ItemKey {
        kind,
        sub_type: text(definition, "SubType"),
        size: integer(definition, "Size"),
        grade: integer(definition, "Grade"),
        manufacturer: reference(definition, "Manufacturer"),
        components: hasher.digest(),
        stats: stats_digest(stats),
    })
}

fn is_cosmetic_component(type_name: &str) -> bool {
    let lower = type_name.to_ascii_lowercase();
    COSMETIC_COMPONENTS
        .iter()
        .any(|cosmetic| lower.contains(cosmetic))
}

fn stats_digest(stats: Option<&ItemStats<'_>>) -> Digest {
    let mut hasher = Wide::new();
    if let Some(stats) = stats {
        let _written = serde_json::to_writer(HasherWriter(&mut hasher), stats);
    }
    hasher.digest()
}

struct HasherWriter<'h, H: Hasher>(&'h mut H);

impl<H: Hasher> io::Write for HasherWriter<'_, H> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
