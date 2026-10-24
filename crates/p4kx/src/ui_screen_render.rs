// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::gfx::{Background, CanvasSize, RasterOptions};
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
    let (width, height) = write(&composed, &raster, options)?;
    let report = composed.report();
    if as_json {
        json::print(&RenderJson {
            screen: options.screen,
            target: json::path(options.out),
            format: if options.svg { "svg" } else { "png" },
            width,
            height,
            report,
        })?;
    } else {
        print_report(options, report, width, height)?;
    }
    Ok(ExitCode::from(if report.is_complete() {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }))
}

fn write(composed: &Composed, raster: &RasterOptions, options: &Options<'_>) -> Result<(u32, u32)> {
    let mut bytes = Vec::new();
    let size = if options.svg {
        let written = composed.write_svg(raster, Some(options.screen), &mut bytes)?;
        (written.width, written.height)
    } else {
        let rendered = composed.rasterize(raster)?;
        rendered.image.write_png(&mut bytes)?;
        (rendered.image.width(), rendered.image.height())
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

fn print_report(options: &Options<'_>, report: &Report, width: u32, height: u32) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
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
