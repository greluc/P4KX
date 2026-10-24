// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use thiserror::Error;

use crate::DataForge;
use crate::record_table::RecordTable;
use crate::rows::{NO_STRING, PropertyDef};
use crate::table::Table;

/// The most issues a [`DataForge`] keeps; [`DataForge::issue_count`] counts all of them.
pub const MAX_DATAFORGE_ISSUES: usize = 1024;

/// A deviation from the known layout that the reader tolerates and reports instead of failing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DataForgeIssue {
    /// A property has bits set above the low byte of its conversion type; they are ignored.
    #[error(
        "property {property} of the DataForge has the conversion flags {flags:#04x}, which are ignored"
    )]
    ConversionFlags {
        /// The position of the property in the property table.
        property: u32,
        /// The high byte of the stored conversion type.
        flags: u8,
    },
    /// A Record has no name; it reads as an empty name.
    #[error("Record {record} of the DataForge has no name")]
    RecordWithoutName {
        /// The position of the Record in definition order.
        record: u32,
    },
    /// A Record has no tag; it reads as no tag.
    #[error("Record {record} of the DataForge has no tag")]
    RecordWithoutTag {
        /// The position of the Record in definition order.
        record: u32,
    },
}

/// The issues found on opening, the first [`MAX_DATAFORGE_ISSUES`] of them, and how many there are.
#[derive(Debug, Default)]
pub(crate) struct Issues {
    pub(crate) list: Vec<DataForgeIssue>,
    pub(crate) total: usize,
}

impl Issues {
    fn push(&mut self, issue: DataForgeIssue) {
        self.total = self.total.saturating_add(1);
        if self.list.len() < MAX_DATAFORGE_ISSUES {
            self.list.push(issue);
        }
    }

    /// Finds the issues of the property and Record tables.
    pub(crate) fn find(properties: Table<'_, PropertyDef>, records: &RecordTable<'_>) -> Self {
        let mut issues = Self::default();
        for (index, def) in properties.iter_indexed() {
            if def.flags != 0 {
                issues.push(DataForgeIssue::ConversionFlags {
                    property: index,
                    flags: def.flags,
                });
            }
        }
        for (index, def) in records.iter_indexed() {
            if def.name == NO_STRING {
                issues.push(DataForgeIssue::RecordWithoutName { record: index });
            }
            if def.tag == Some(NO_STRING) {
                issues.push(DataForgeIssue::RecordWithoutTag { record: index });
            }
        }
        issues
    }
}

impl DataForge<'_> {
    /// Returns the first [`MAX_DATAFORGE_ISSUES`] deviations from the known layout that opening tolerated, in
    /// table order.
    #[must_use]
    pub fn issues(&self) -> &[DataForgeIssue] {
        &self.issues.list
    }

    /// Returns the number of deviations that opening tolerated, which may exceed
    /// [`MAX_DATAFORGE_ISSUES`].
    #[must_use]
    pub const fn issue_count(&self) -> usize {
        self.issues.total
    }
}
