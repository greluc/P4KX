// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use p4kx_export::{
    CatalogError, CatalogFailure, CatalogFile, CatalogOptions, CatalogSummary, Compression,
    CraftingWarning, Filter, LoadoutSummary, PaintSummary, Playability, ShopSummary,
    StarmapSummary, StoreSlugSummary, catalog_schema, write_catalog,
};
use p4kx_extraction::Archive;

use super::catalog_json::summary_json;
use super::catalog_size::warn_about_size;
use crate::catalog_input::{InvalidExportTime, MissingCatalogArgs, refuse_gzip};
use crate::cli::{CatalogArgs, CatalogCommand};
use crate::exit;
use crate::extract::cancel_on_interrupt;
use crate::json;
use crate::loc::print_notes;
use crate::progress::ProgressLine;

const STALE_SLUGS_SHOWN: usize = 5;

/// Writes the Basetool catalog of the Archive to the output file and prints its summary, or
/// prints the catalog's JSON Schema; the exit code is 1 when a selected Record could not be read.
pub(crate) fn run(args: &CatalogArgs, as_json: bool) -> Result<ExitCode> {
    if matches!(args.command, Some(CatalogCommand::Schema)) {
        print_schema()?;
        return Ok(ExitCode::from(exit::SUCCESS));
    }
    let (Some(archive), Some(out)) = (&args.archive, &args.out) else {
        return Err(MissingCatalogArgs.into());
    };
    refuse_gzip(out)?;
    let exported_at = match args.exported_at {
        None => SystemTime::now(),
        Some(seconds) => UNIX_EPOCH
            .checked_add(Duration::from_secs(seconds))
            .ok_or_else(|| InvalidExportTime::of(seconds))?,
    };
    let archive = crate::key::open(archive)?;
    let filter = args.filter.as_deref().map(Filter::new);
    let cancel = cancel_on_interrupt()?;
    let line = ProgressLine::new();
    let show = |progress| line.show_records(progress);
    let options = CatalogOptions {
        filter: filter.as_ref(),
        exported_at,
        cancel: Some(&cancel),
        progress: Some(&show),
    };
    crate::guard::ensure_writable(out)?;
    let written = write_file(&archive, &options, out);
    line.finish();
    let summary = &written?;
    print_notes(summary.text_notes());
    print_failures(summary.failures())?;
    print_warnings(summary.warnings())?;
    warn_about_size(summary)?;
    let printed = if as_json {
        json::print(&summary_json(&archive, out, summary))
    } else {
        print_summary(summary, out)
    };
    if let Err(error) = printed
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }
    Ok(ExitCode::from(if summary.is_complete() {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }))
}

/// Builds the catalog and streams it into a temporary file next to `out`, compressed when the name
/// ends in `.zst`, then moves it over `out`; a failure or an interrupt leaves `out` as it was.
fn write_file(
    archive: &Archive,
    options: &CatalogOptions<'_>,
    out: &Path,
) -> Result<CatalogSummary> {
    let written = write_catalog(archive, options, Compression::for_path(out), || {
        CatalogFile::create(out).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("cannot write {}: {error}", out.display()),
            )
        })
    });
    let (summary, file) = written.map_err(into_anyhow)?;
    file.commit()
        .with_context(|| format!("cannot write {}", out.display()))?;
    Ok(summary)
}

fn print_failures(failures: &[CatalogFailure]) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for failure in failures {
        writeln!(
            err,
            "failed {} ({}): {}",
            failure.record_path, failure.guid, failure.reason
        )?;
    }
    Ok(())
}

fn print_warnings(warnings: &[CraftingWarning]) -> io::Result<()> {
    let mut err = io::stderr().lock();
    for warning in warnings {
        writeln!(err, "warning: {warning}")?;
    }
    Ok(())
}

fn print_schema() -> Result<()> {
    let mut text = serde_json::to_string_pretty(&catalog_schema())?;
    text.push('\n');
    io::stdout().lock().write_all(text.as_bytes())?;
    Ok(())
}

fn into_anyhow(error: CatalogError) -> anyhow::Error {
    match error {
        CatalogError::Load(error) => error.into(),
        CatalogError::LoadTexts(error) => error.into(),
        CatalogError::DataForge(error) => error.into(),
        other => other.into(),
    }
}

fn print_classification(stdout: &mut impl Write, summary: &CatalogSummary) -> io::Result<()> {
    let playability = summary.playability();
    for (kind, counts) in [("items", playability.items), ("ships", playability.ships)] {
        writeln!(
            stdout,
            "{kind} playability: {} player, {} ai, {} mission, {} special, {} unclassified",
            counts.of(Playability::Player),
            counts.of(Playability::Ai),
            counts.of(Playability::Mission),
            counts.of(Playability::Special),
            counts.unclassified()
        )?;
    }
    let variants = summary.variants();
    for (kind, counts) in [("items", variants.items), ("ships", variants.ships)] {
        writeln!(
            stdout,
            "{kind} variants: {} in {} groups",
            counts.variants, counts.groups
        )?;
    }
    Ok(())
}

fn print_starmap(out: &mut impl Write, starmap: StarmapSummary) -> io::Result<()> {
    writeln!(
        out,
        "starmap placement: {} of {} objects have a position, {} are placed apart, {} systems, {} containers unreadable, {} missing, {} children malformed",
        starmap.placed,
        starmap.objects,
        starmap.ambiguous,
        starmap.systems,
        starmap.unreadable_containers,
        starmap.missing_containers,
        starmap.malformed_children
    )
}

fn print_paints(out: &mut impl Write, paints: PaintSummary) -> io::Result<()> {
    writeln!(
        out,
        "paint assignment: {} of {} paints fit a ship, {} fit none, {} ships have paints, {} vehicle files unreadable",
        paints.assigned, paints.paints, paints.unassigned, paints.ships, paints.unreadable_vehicles
    )
}

fn print_store_slugs(out: &mut impl Write, slugs: &StoreSlugSummary) -> io::Result<()> {
    if slugs.stale_check_skipped {
        return writeln!(
            out,
            "ship store slugs: no catalog ship matches a store slug, stale check skipped"
        );
    }
    writeln!(
        out,
        "ship store slugs: {} of {} slugs map to a ship, {} resolve, {} retired, {} unmapped, {} stale",
        slugs.mapped,
        slugs.slugs,
        slugs.resolved,
        slugs.retired,
        slugs.unmapped,
        slugs.stale.len()
    )?;
    if !slugs.stale.is_empty() {
        let shown = slugs
            .stale
            .iter()
            .take(STALE_SLUGS_SHOWN)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        let more = slugs.stale.len().saturating_sub(STALE_SLUGS_SHOWN);
        if more == 0 {
            writeln!(out, "stale ship store slugs: {shown}")?;
        } else {
            writeln!(out, "stale ship store slugs: {shown} and {more} more")?;
        }
    }
    Ok(())
}

fn print_shops(out: &mut impl Write, shops: &ShopSummary) -> io::Result<()> {
    writeln!(
        out,
        "shops: {} of {} registry entries, {} malformed, {} repeated, {} with a kind, {} linked to a starmap object, {} not, {} commodity exchanges, {} containers unreadable, {} with an item list ({} ambiguous, {} unscanned, {} placeholders, {} files missing, {} files unreadable), registry {}",
        shops.shops,
        shops.registry_entries,
        shops.malformed,
        shops.duplicate,
        shops.with_kind,
        shops.linked,
        shops.unlinked,
        shops.commodity_exchanges,
        shops.containers_unreadable,
        shops.with_items,
        shops.inventory_ambiguous,
        shops.inventory_unscanned,
        shops.inventory_placeholder,
        shops.inventories_missing,
        shops.inventories_unreadable,
        if shops.registry_unreadable {
            "unreadable"
        } else if shops.registry_missing {
            "missing"
        } else {
            "read"
        }
    )
}

fn print_summary(summary: &CatalogSummary, out: &Path) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    for kind in summary.kinds() {
        writeln!(
            stdout,
            "{}: {} written, {} skipped, {}",
            kind.kind,
            kind.count,
            kind.skipped,
            if kind.complete { "complete" } else { "partial" }
        )?;
    }
    let classes = summary.item_classes();
    writeln!(
        stdout,
        "item classes: {} read, {} other values, {} without a Class: line or English description",
        classes.classified, classes.other, classes.missing
    )?;
    print_paints(&mut stdout, summary.paints())?;
    print_loadouts(&mut stdout, summary.loadouts())?;
    print_starmap(&mut stdout, summary.starmap())?;
    print_shops(&mut stdout, summary.shops())?;
    print_store_slugs(&mut stdout, summary.store_slugs())?;
    print_classification(&mut stdout, summary)?;
    for text in summary.texts() {
        writeln!(
            stdout,
            "{}: {} texts resolved, {} unresolved, {} placeholders{}",
            text.language,
            text.resolved,
            text.unresolved,
            text.placeholder,
            if text.available {
                ""
            } else {
                ", the Archive holds no global.ini"
            }
        )?;
    }
    writeln!(
        stdout,
        "wrote {}: {}, {} Records skipped, {} failed",
        out.display(),
        size_text(summary),
        summary.skipped(),
        summary.failures().len()
    )
}

fn print_loadouts(out: &mut impl Write, loadouts: LoadoutSummary) -> io::Result<()> {
    writeln!(
        out,
        "loadouts: {} ships and {} items, {} ports, {} hold an Entity ({} docked vehicles), {} unresolved, {} Entities without an entry in items or ships, {} loadouts with issues",
        loadouts.ships,
        loadouts.items,
        loadouts.ports,
        loadouts.attached,
        loadouts.docked_vehicles,
        loadouts.unresolved,
        loadouts.outside,
        loadouts.with_issues
    )
}

fn size_text(summary: &CatalogSummary) -> String {
    summary.compressed_size().map_or_else(
        || format!("{} bytes", summary.size()),
        |compressed| {
            format!(
                "{compressed} bytes zstd-compressed from {} bytes",
                summary.size()
            )
        },
    )
}
