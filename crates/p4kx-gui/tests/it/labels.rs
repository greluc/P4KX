// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Label queries for tests whose window also shows a path, which a substring match on a common
//! word would find a second time.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use crate::node_text::node_text;

/// How many labels of `harness` begin with `prefix`; unlike a substring query, a path further
/// along in a label cannot match.
pub(crate) fn labels_starting_with<S>(harness: &Harness<'_, S>, prefix: &str) -> usize {
    harness
        .query_all_by_label_contains(prefix)
        .filter_map(|node| node_text(&node))
        .filter(|text| text.starts_with(prefix))
        .count()
}
