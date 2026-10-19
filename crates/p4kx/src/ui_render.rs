use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::gfx::{Background, CanvasSize, RasterOptions, Skipped};
use p4kx_extraction::{UiMovie, UiRender};
use serde::Serialize;

use crate::ui::chain;
use crate::{exit, json};

#[derive(Serialize)]
struct RenderJson<'a> {
    movie: &'a str,
    target: String,
    width: u32,
    height: u32,
    skipped: Vec<SkippedJson>,
    failed: Vec<FailedJson>,
}

#[derive(Serialize)]
struct SkippedJson {
    id: u16,
    depth: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    sprite: Option<u16>,
    reason: &'static str,
    drawn: bool,
    message: String,
}

#[derive(Serialize)]
struct FailedJson {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<usize>,
    error: String,
}

/// Runs `p4kx ui render` on an opened movie.
pub(crate) fn render(
    movie: &UiMovie<'_>,
    out: &Path,
    size: Option<CanvasSize>,
    background: Background,
    as_json: bool,
) -> Result<ExitCode> {
    let options = RasterOptions {
        size: size.unwrap_or(CanvasSize::Stage),
        background,
    };
    let render = movie.render_to_file(&options, out)?;
    let skipped: Vec<SkippedJson> = render.skipped.iter().map(skipped_json).collect();
    let failed = failed(&render);
    if as_json {
        json::print(&RenderJson {
            movie: movie.name(),
            target: json::path(out),
            width: render.image.width(),
            height: render.image.height(),
            skipped,
            failed,
        })?;
    } else {
        print_report(movie, out, &render, &skipped, &failed)?;
    }
    Ok(ExitCode::from(if render.is_complete() {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }))
}

fn print_report(
    movie: &UiMovie<'_>,
    out: &Path,
    render: &UiRender,
    skipped: &[SkippedJson],
    failed: &[FailedJson],
) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    for entry in skipped {
        let place = entry.sprite.map_or_else(
            || format!("depth {}", entry.depth),
            |sprite| format!("depth {} of sprite {sprite}", entry.depth),
        );
        let action = if entry.drawn {
            "drawn without an effect"
        } else {
            "skipped"
        };
        writeln!(
            stderr,
            "{action}: definition {} at {place}: {}",
            entry.id, entry.message
        )?;
    }
    for failure in failed {
        match (failure.code, failure.offset) {
            (Some(code), Some(offset)) => writeln!(
                stderr,
                "error: unreadable tag {code} at offset {offset}: {}",
                failure.error
            )?,
            _ => writeln!(stderr, "error: {}", failure.error)?,
        }
    }
    drop(stderr);
    writeln!(
        io::stdout().lock(),
        "rendered {} into {}, {} x {} px; {} elements left out, {} bitmaps and {} tags unreadable",
        movie.name(),
        out.display(),
        render.image.width(),
        render.image.height(),
        skipped.len(),
        render.bitmap_failures.len(),
        render.tag_failures.len()
    )
}

fn skipped_json(skipped: &Skipped) -> SkippedJson {
    SkippedJson {
        id: skipped.id,
        depth: skipped.depth,
        sprite: skipped.sprite,
        reason: skipped.reason.code(),
        drawn: skipped.reason.is_effect(),
        message: skipped.reason.to_string(),
    }
}

fn failed(render: &UiRender) -> Vec<FailedJson> {
    let tags = render.tag_failures.iter().map(|failure| FailedJson {
        kind: "tag",
        id: None,
        code: Some(failure.code),
        offset: Some(failure.offset),
        error: chain(&failure.error),
    });
    let bitmaps = render.bitmap_failures.iter().map(|failure| FailedJson {
        kind: "bitmap",
        id: Some(failure.id),
        code: None,
        offset: None,
        error: chain(&failure.error),
    });
    tags.chain(bitmaps).collect()
}
