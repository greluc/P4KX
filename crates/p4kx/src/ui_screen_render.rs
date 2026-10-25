// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::gfx::{Background, CanvasSize, RasterOptions, Skipped};
use p4kx_extraction::{DataForgeEntry, Language};
use p4kx_screens::{Report, ScreenTexts, Still, StillFormat, StillRequest};
use serde::Serialize;

use crate::{exit, json};

pub(crate) struct Options<'a> {
    pub(crate) screen: &'a str,
    pub(crate) brand: Option<&'a str>,
    pub(crate) lang: Option<Language>,
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
    let texts = ScreenTexts::load(
        &archive,
        &forge,
        Some(options.lang.unwrap_or(Language::English)),
    );
    for warning in texts.warnings() {
        warn(warning);
    }
    let request = StillRequest {
        screen: options.screen,
        brand: options.brand,
        raster: RasterOptions {
            size: options.size.unwrap_or(CanvasSize::Stage),
            background: options.background,
        },
        format: if options.svg {
            StillFormat::Svg
        } else {
            StillFormat::Png
        },
    };
    let still = Still::render(&forge, &texts, &request)?;
    for warning in still.warnings() {
        warn(warning);
    }
    still.write(options.out)?;
    let skipped = skipped_json(still.skipped());
    if as_json {
        json::print(&RenderJson {
            screen: options.screen,
            target: json::path(options.out),
            format: if options.svg { "svg" } else { "png" },
            width: still.width(),
            height: still.height(),
            report: still.report(),
            skipped: &skipped,
        })?;
    } else {
        print_report(options, &still, &skipped)?;
    }
    Ok(ExitCode::from(if still.is_complete() {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }))
}

fn warn(error: &impl std::fmt::Display) {
    drop(writeln!(
        io::stderr().lock(),
        "warning: texts are drawn without it: {error}"
    ));
}

fn print_report(options: &Options<'_>, still: &Still, skipped: &[SkippedJson]) -> io::Result<()> {
    let report = still.report();
    let (width, height) = (still.width(), still.height());
    let mut stderr = io::stderr().lock();
    for skipped in skipped {
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
        "rendered {} into {}, {width} x {height} px; {} widgets in {} canvases, {} shapes, {} texts drawn and {} left out, {} kinds of element left out",
        options.screen,
        options.out.display(),
        report.widgets,
        report.canvases,
        report.shapes,
        report.texts_drawn,
        report.texts_left_out,
        report.unsupported.len()
    )
}
