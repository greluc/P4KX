// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use p4kx_assembly::OutfitReport;
use serde::Serialize;

use crate::query_table::one_line;

/// The outfits of an Entity as the reports show them.
#[derive(Debug, Serialize)]
pub(crate) struct OutfitsJson {
    base_files: Vec<String>,
    outfits: Vec<OutfitJson>,
}

#[derive(Debug, Serialize)]
struct OutfitJson {
    tags: Vec<String>,
    files: Vec<String>,
    applied: bool,
}

impl From<&OutfitReport> for OutfitsJson {
    fn from(report: &OutfitReport) -> Self {
        Self {
            base_files: report.base_files().to_vec(),
            outfits: report
                .outfits()
                .iter()
                .map(|outfit| OutfitJson {
                    tags: outfit.tags().to_vec(),
                    files: outfit.files().to_vec(),
                    applied: outfit.is_applied(),
                })
                .collect(),
        }
    }
}

/// Writes one line for the base Loadout and one for each outfit, with whether it is worn.
pub(crate) fn print(out: &mut impl Write, report: &OutfitsJson) -> io::Result<()> {
    if !report.base_files.is_empty() {
        writeln!(out, "outfit base: {}", files_text(&report.base_files))?;
    }
    for (at, outfit) in report.outfits.iter().enumerate() {
        let state = if outfit.applied { "worn" } else { "not worn" };
        let tags = if outfit.tags.is_empty() {
            "no name tags".to_owned()
        } else {
            format!("name tags {}", outfit.tags.join(", "))
        };
        writeln!(
            out,
            "outfit {} {state}, {}: {}",
            at.saturating_add(1),
            one_line(&tags),
            files_text(&outfit.files)
        )?;
    }
    Ok(())
}

fn files_text(files: &[String]) -> String {
    if files.is_empty() {
        "-".to_owned()
    } else {
        one_line(&files.join(", "))
    }
}
