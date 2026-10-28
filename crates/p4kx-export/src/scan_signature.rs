// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{Record, RecordTree, TreeBuilder, Value};
use rayon::prelude::*;

use crate::CatalogFailure;
use crate::record_fields::{elements, locale_key, structure};

const RADAR_GLOBAL_PARAMS: &str = "RadarSystemGlobalParams";
const SCAN_SIGNATURE_KEY: &str = "@hud_scanning_info_rs_signature";

/// The index in an entity's `signatures` that holds the scan signature.
///
/// The `signatureTypeParams` array of the `RadarSystemGlobalParams` Record has one entry per slot
/// of `signatures`; the slot whose `displayName` is `@hud_scanning_info_rs_signature` is the scan
/// signature.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SignatureSlot(Option<usize>);

impl SignatureSlot {
    /// Reads the slot from the `RadarSystemGlobalParams` Record among `records`; a Record that
    /// cannot be read is returned as a failure.
    pub(crate) fn read(records: &[Record<'_>]) -> (Self, Vec<CatalogFailure>) {
        let found: Vec<Result<Self, CatalogFailure>> = records
            .par_iter()
            .map_init(TreeBuilder::new, |trees, record| {
                if record.type_name().ok()? != RADAR_GLOBAL_PARAMS {
                    return None;
                }
                Some(
                    trees
                        .build(record)
                        .map(|tree| Self::of_tree(&tree))
                        .map_err(|source| CatalogFailure::of(record, &source)),
                )
            })
            .flatten()
            .collect();
        let mut slot = Self::default();
        let mut failures = Vec::new();
        for result in found {
            match result {
                Ok(found) if slot.0.is_none() => slot = found,
                Ok(_) => {}
                Err(failure) => failures.push(failure),
            }
        }
        (slot, failures)
    }

    fn of_tree(tree: &RecordTree<'_>) -> Self {
        let system = structure(Some(tree.root()), "signatureSystemParams");
        Self(
            elements(system, "signatureTypeParams")
                .iter()
                .position(|entry| {
                    matches!(
                        entry,
                        Value::Struct(entry)
                            if locale_key(Some(entry), "displayName") == Some(SCAN_SIGNATURE_KEY)
                    )
                }),
        )
    }

    /// Returns the scan signature among `signatures`; `None` when the slot is unknown or out of
    /// range, or when every signature is zero.
    pub(crate) fn value(self, signatures: &[f32]) -> Option<f32> {
        if signatures.iter().all(|signature| *signature == 0.0) {
            return None;
        }
        signatures.get(self.0?).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slot_picks_its_signature() {
        let slot = SignatureSlot(Some(2));
        assert_eq!(slot.value(&[0.0, 0.0, 3170.0, 0.0]), Some(3170.0));
    }

    #[test]
    fn an_unknown_slot_or_one_beyond_the_array_picks_nothing() {
        assert_eq!(SignatureSlot::default().value(&[0.0, 5.0]), None);
        assert_eq!(SignatureSlot(Some(4)).value(&[0.0, 5.0]), None);
    }

    #[test]
    fn all_zero_signatures_pick_nothing() {
        assert_eq!(SignatureSlot(Some(1)).value(&[0.0, 0.0, 0.0]), None);
        assert_eq!(SignatureSlot(Some(1)).value(&[]), None);
    }

    #[test]
    fn a_zero_slot_among_other_signatures_is_zero() {
        assert_eq!(SignatureSlot(Some(1)).value(&[7.0, 0.0]), Some(0.0));
    }
}
