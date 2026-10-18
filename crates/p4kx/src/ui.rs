use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::{Archive, UiExtraction, UiMovie};
use serde::Serialize;

use crate::cli::{UiAction, UiArgs};
use crate::ui_json::{AssetsJson, describe};
use crate::{exit, json};

const NONE: &str = "-";

#[derive(Serialize)]
struct ExtractJson<'a> {
    movie: &'a str,
    target: String,
    extracted: Vec<WrittenJson>,
    failed: Vec<FailedJson>,
}

#[derive(Serialize)]
struct WrittenJson {
    kind: &'static str,
    id: u16,
    file: String,
    size: u64,
}

#[derive(Serialize)]
struct FailedJson {
    kind: &'static str,
    id: u16,
    error: String,
}

/// Runs `p4kx ui assets` or `p4kx ui extract`.
pub(crate) fn run(args: &UiArgs, as_json: bool) -> Result<ExitCode> {
    let source = match &args.action {
        UiAction::Assets(source) | UiAction::Extract { source, .. } => source,
    };
    let archive;
    let movie = match &source.entry {
        Some(entry) => {
            archive = Archive::open(&source.path)?;
            UiMovie::from_archive(&archive, entry)?
        }
        None => UiMovie::from_file(&source.path)?,
    };
    match &args.action {
        UiAction::Assets(_) => {
            let assets = movie.movie().assets();
            let described = describe(&movie, &assets);
            if as_json {
                json::print(&described)?;
            } else {
                print_assets(&described)?;
            }
            Ok(ExitCode::from(exit::SUCCESS))
        }
        UiAction::Extract { out, .. } => extract(&movie, out, as_json),
    }
}

fn print_assets(assets: &AssetsJson<'_>) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    writeln!(out, "Movie:        {}", assets.movie)?;
    writeln!(
        out,
        "Format:       {} version {}, {}, {} bytes uncompressed",
        assets.signature,
        assets.version,
        match assets.compression {
            "none" => "not compressed",
            "zlib" => "zlib-compressed",
            "lzma" => "LZMA-compressed",
            other => other,
        },
        assets.length
    )?;
    writeln!(
        out,
        "Stage:        {} x {} px, {} fps, {} frames",
        pixels(assets.stage.width()),
        pixels(assets.stage.height()),
        assets.frame_rate,
        assets.frame_count
    )?;
    if let Some(exporter) = &assets.exporter {
        writeln!(
            out,
            "Exporter:     GFx {:#06X}, from {}",
            exporter.version, exporter.swf_name
        )?;
    }
    let tags: u32 = assets.tags.iter().map(|tag| tag.count).sum();
    writeln!(
        out,
        "Tags:         {tags}, {} unknown, {} unreadable",
        assets.unknown_tags.len(),
        assets.failures.len()
    )?;
    writeln!(out)?;
    writeln!(out, "{:>6}  {:<30}  {:>7}", "CODE", "TAG", "COUNT")?;
    for tag in &assets.tags {
        let name = tag.name.unwrap_or("unknown");
        writeln!(out, "{:>6}  {name:<30}  {:>7}", tag.code, tag.count)?;
    }
    print_definitions(&mut out, assets)?;
    for tag in &assets.unknown_tags {
        writeln!(
            out,
            "unknown tag {} at offset {}, {} bytes",
            tag.code, tag.offset, tag.length
        )?;
    }
    for failure in &assets.failures {
        writeln!(
            out,
            "unreadable tag {} at offset {}: {}",
            failure.code, failure.offset, failure.error
        )?;
    }
    out.flush()
}

fn print_definitions(out: &mut impl Write, assets: &AssetsJson<'_>) -> io::Result<()> {
    section(
        out,
        "Shapes",
        assets.shapes.len() + assets.morph_shapes.len(),
    )?;
    for (kind, shapes) in [
        ("shape", &assets.shapes),
        ("morph shape", &assets.morph_shapes),
    ] {
        for shape in shapes {
            writeln!(
                out,
                "{:>6}  {kind:<12} v{}  {} x {} px, {} edges  {}",
                shape.id,
                shape.version,
                pixels(shape.bounds.width()),
                pixels(shape.bounds.height()),
                shape.edges,
                shape.export_name.unwrap_or(NONE)
            )?;
        }
    }
    print_bitmaps(out, assets)?;
    section(out, "Fonts", assets.fonts.len())?;
    for font in &assets.fonts {
        writeln!(
            out,
            "{:>6}  {:<20} {:>5} glyphs  {}",
            font.id, font.format, font.glyphs, font.name
        )?;
    }
    section(out, "Sprites", assets.sprites.len())?;
    for sprite in &assets.sprites {
        writeln!(
            out,
            "{:>6}  {:>5} frames, {} placed  {}",
            sprite.id,
            sprite.frames,
            sprite.placed.len(),
            sprite.export_name.unwrap_or(NONE)
        )?;
    }
    Ok(())
}

fn print_bitmaps(out: &mut impl Write, assets: &AssetsJson<'_>) -> io::Result<()> {
    section(out, "Bitmaps", assets.bitmaps.len())?;
    for bitmap in &assets.bitmaps {
        let size = match (bitmap.width, bitmap.height) {
            (Some(width), Some(height)) => format!("{width} x {height}"),
            _ => NONE.to_owned(),
        };
        let detail = match (bitmap.file, bitmap.image_id) {
            (Some(file), _) => file.to_owned(),
            (None, Some(image)) => format!("of bitmap {image}"),
            (None, None) => bitmap.format.clone().unwrap_or_default(),
        };
        writeln!(
            out,
            "{:>6}  {:<12} {size:>11}  {detail}  {}",
            bitmap.id,
            bitmap.kind,
            bitmap.export_name.unwrap_or(NONE)
        )?;
    }
    Ok(())
}

fn pixels(twips: i64) -> String {
    let whole = twips / 20;
    match twips % 20 {
        0 => whole.to_string(),
        rest => format!(
            "{}{whole}.{:02}",
            if twips < 0 && whole == 0 { "-" } else { "" },
            rest.abs() * 5
        ),
    }
}

fn section(out: &mut impl Write, title: &str, count: usize) -> io::Result<()> {
    writeln!(out)?;
    writeln!(out, "{title}: {count}")
}

fn extract(movie: &UiMovie<'_>, out: &Path, as_json: bool) -> Result<ExitCode> {
    let extraction = movie.extract_assets(out)?;
    if as_json {
        json::print(&extract_json(movie, out, &extraction))?;
    } else {
        let mut stderr = io::stderr().lock();
        for failure in &extraction.failures {
            writeln!(stderr, "error: {}", chain(&failure.error))?;
        }
        writeln!(
            io::stdout().lock(),
            "extracted {} of {} definitions from {} into {}",
            extraction.written.len(),
            extraction.written.len() + extraction.failures.len(),
            movie.name(),
            out.display()
        )?;
    }
    Ok(ExitCode::from(if extraction.failures.is_empty() {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }))
}

fn extract_json<'a>(
    movie: &'a UiMovie<'_>,
    out: &Path,
    extraction: &UiExtraction,
) -> ExtractJson<'a> {
    ExtractJson {
        movie: movie.name(),
        target: json::path(out),
        extracted: extraction
            .written
            .iter()
            .map(|written| WrittenJson {
                kind: written.kind.name(),
                id: written.id,
                file: json::path(&written.path),
                size: written.size,
            })
            .collect(),
        failed: extraction
            .failures
            .iter()
            .map(|failure| FailedJson {
                kind: failure.kind.name(),
                id: failure.id,
                error: chain(&failure.error),
            })
            .collect(),
    }
}

/// Returns an error and its causes joined by `: `.
pub(crate) fn chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut text = String::new();
    let mut current = Some(error);
    while let Some(error) = current {
        if !text.is_empty() {
            text.push_str(": ");
        }
        text.push_str(&error.to_string());
        current = error.source();
    }
    text
}
