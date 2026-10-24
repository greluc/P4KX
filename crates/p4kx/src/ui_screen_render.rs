// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::gfx::{Background, CanvasSize, RasterOptions, Skipped};
use p4kx_extraction::{ConversionError, DataForgeEntry, write_replacing};
use p4kx_screens::{Composed, Report, ScreenComposer};
use serde::Serialize;

use crate::{exit, json};

pub(crate) struct Options<'a> {
    pub(crate) screen: &'a str,
    pub(crate) brand: Option<&'a str>,
    pub(crate) out: &'a Path,
    pub(crate) size: Option<CanvasSize>,
    pub(crate) background: Background,
    pub(crate) svg: bool,
}

#[derive(Serialize)]
struct RenderJson<'a> {
    screen: &'a str,
    target: String,
    format: &'static str,
    width: u32,
    height: u32,
    #[serde(flatten)]
    report: &'a Report,
    skipped: &'a [SkippedJson],
}

#[derive(Serialize)]
struct SkippedJson {
    reason: &'static str,
    message: String,
}

struct Written {
    width: u32,
    height: u32,
    skipped: Vec<SkippedJson>,
}

fn skipped_json(skipped: &[Skipped]) -> Vec<SkippedJson> {
    skipped
        .iter()
        .map(|skipped| SkippedJson {
            reason: skipped.reason.code(),
            message: skipped.reason.to_string(),
        })
        .collect()
}

/// Runs `p4kx ui render --screen`.
pub(crate) fn run(archive: &Path, options: &Options<'_>, as_json: bool) -> Result<ExitCode> {
    let archive = crate::key::open(archive)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let composed = ScreenComposer::new(&forge).compose(options.screen, options.brand)?;
    let raster = RasterOptions {
        size: options.size.unwrap_or(CanvasSize::Stage),
        background: options.background,
    };
    let written = write(&composed, &raster, options)?;
    let report = composed.report();
    if as_json {
        json::print(&RenderJson {
            screen: options.screen,
            target: json::path(options.out),
            format: if options.svg { "svg" } else { "png" },
            width: written.width,
            height: written.height,
            report,
            skipped: &written.skipped,
        })?;
    } else {
        print_report(options, report, &written)?;
    }
    Ok(ExitCode::from(
        if report.is_complete() && written.skipped.is_empty() {
            exit::SUCCESS
        } else {
            exit::PARTIAL_FAILURE
        },
    ))
}

fn write(composed: &Composed, raster: &RasterOptions, options: &Options<'_>) -> Result<Written> {
    let mut bytes = Vec::new();
    let size = if options.svg {
        let written = composed.write_svg(raster, Some(options.screen), &mut bytes)?;
        Written {
            width: written.width,
            height: written.height,
            skipped: skipped_json(&written.skipped),
        }
    } else {
        let rendered = composed.rasterize(raster)?;
        rendered.image.write_png(&mut bytes)?;
        Written {
            width: rendered.image.width(),
            height: rendered.image.height(),
            skipped: skipped_json(&rendered.skipped),
        }
    };
    write_replacing(options.out, |writer| {
        writer
            .write_all(&bytes)
            .map_err(|source| ConversionError::Write {
                path: options.out.to_owned(),
                source,
            })
    })?;
    Ok(size)
}

fn print_report(options: &Options<'_>, report: &Report, written: &Written) -> io::Result<()> {
    let (width, height) = (written.width, written.height);
    let mut stderr = io::stderr().lock();
    for skipped in &written.skipped {
        writeln!(stderr, "skipped by the renderer: {}", skipped.message)?;
    }
    for entry in &report.unsupported {
        writeln!(
            stderr,
            "left out: {} x{}: {} ({})",
            entry.element,
            entry.count,
            entry.reason,
            entry.examples.join(", ")
        )?;
    }
    for canvas in &report.missing_canvases {
        writeln!(stderr, "canvas not drawn: {canvas}")?;
    }
    if report.truncated {
        writeln!(
            stderr,
            "stopped early: a limit of the composition was reached"
        )?;
    }
    drop(stderr);
    writeln!(
        io::stdout().lock(),
        "rendered {} into {}, {width} x {height} px; {} widgets in {} canvases, {} shapes, {} kinds of element left out",
        options.screen,
        options.out.display(),
        report.widgets,
        report.canvases,
        report.shapes,
        report.unsupported.len()
    )
}
