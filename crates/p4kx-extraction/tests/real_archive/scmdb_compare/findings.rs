// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::BTreeMap;

/// Relative tolerance between an SCMDB number and the `f32` the DataForge stores.
const RELATIVE_TOLERANCE: f64 = 1e-5;

/// Absolute tolerance for numbers near zero.
const ABSOLUTE_TOLERANCE: f64 = 1e-6;

/// The checks made per category and the disagreements found.
#[derive(Debug, Default)]
pub(crate) struct Findings {
    checks: BTreeMap<&'static str, usize>,
    mismatches: BTreeMap<&'static str, Vec<String>>,
}

impl Findings {
    /// Counts one check of `category` and records `describe()` when the values disagree.
    pub(crate) fn check(
        &mut self,
        category: &'static str,
        agrees: bool,
        describe: impl FnOnce() -> String,
    ) {
        *self.checks.entry(category).or_default() += 1;
        if !agrees {
            self.mismatches
                .entry(category)
                .or_default()
                .push(describe());
        }
    }

    /// Checks that two numbers are equal within the tolerance; two missing numbers agree.
    pub(crate) fn number(
        &mut self,
        category: &'static str,
        ident: &str,
        field: &str,
        scmdb: Option<f64>,
        p4kx: Option<f64>,
    ) {
        let agrees = match (scmdb, p4kx) {
            (Some(scmdb), Some(p4kx)) => {
                (scmdb - p4kx).abs()
                    <= ABSOLUTE_TOLERANCE.max(RELATIVE_TOLERANCE * scmdb.abs().max(p4kx.abs()))
            }
            (scmdb, p4kx) => scmdb.is_none() && p4kx.is_none(),
        };
        self.check(category, agrees, || {
            format!("{ident}: {field} SCMDB={scmdb:?} p4kx={p4kx:?}")
        });
    }

    /// Returns whether every check agreed.
    pub(crate) fn is_empty(&self) -> bool {
        self.mismatches.is_empty()
    }

    /// Returns the number of disagreements of `category`.
    pub(crate) fn mismatches_of(&self, category: &str) -> usize {
        self.mismatches.get(category).map_or(0, Vec::len)
    }

    /// Returns one line per category with its checks and disagreements.
    pub(crate) fn summary(&self) -> Vec<String> {
        self.checks
            .iter()
            .map(|(category, checks)| {
                format!(
                    "{category}: {checks} checks, {} disagree",
                    self.mismatches_of(category)
                )
            })
            .collect()
    }

    /// Returns up to `per_category` disagreements of every category.
    pub(crate) fn examples(&self, per_category: usize) -> Vec<&str> {
        self.mismatches
            .values()
            .flat_map(|lines| lines.iter().take(per_category).map(String::as_str))
            .collect()
    }
}
