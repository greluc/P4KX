use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;
use p4kx_extraction::{Archive, ConversionError, Texture, TextureError};
use serde::Serialize;

use crate::cli::{TextureAction, TextureArgs, TextureSource};
use crate::json;

const UNKNOWN: &str = "unknown";

#[derive(Serialize)]
struct InfoJson<'a> {
    texture: &'a str,
    format: String,
    width: u32,
    height: u32,
    depth: u32,
    mip_count: u32,
    array_size: u32,
    faces: u32,
    cubemap: bool,
    parts: Option<u32>,
}

#[derive(Serialize)]
struct MergeJson<'a> {
    texture: &'a str,
    output: String,
    parts: u32,
    size: u64,
}

#[derive(Serialize)]
struct PngJson<'a> {
    texture: &'a str,
    output: String,
    format: &'a str,
    mip: u32,
    width: u32,
    height: u32,
}

/// Runs `p4kx texture info`, `merge` or `png`.
pub(crate) fn run(args: &TextureArgs, as_json: bool) -> Result<()> {
    match &args.action {
        TextureAction::Info(source) => with_texture(source, |texture| info(texture, as_json)),
        TextureAction::Merge { source, out } => {
            with_texture(source, |texture| merge(texture, out, as_json))
        }
        TextureAction::Png {
            source,
            out,
            mip,
            tone_map,
        } => with_texture(source, |texture| {
            png(texture, out, *mip, *tone_map, as_json)
        }),
    }
}

fn with_texture(
    source: &TextureSource,
    run: impl FnOnce(&Texture<'_>) -> Result<()>,
) -> Result<()> {
    match &source.entry {
        Some(entry) => {
            let archive = Archive::open(&source.path)?;
            run(&Texture::from_archive(&archive, entry)?)
        }
        None => run(&Texture::from_file(&source.path)?),
    }
}

fn info(texture: &Texture<'_>, as_json: bool) -> Result<()> {
    let header = texture.header();
    let info = InfoJson {
        texture: texture.name(),
        format: header.format().to_string(),
        width: header.width(),
        height: header.height(),
        depth: header.depth(),
        mip_count: header.mip_count(),
        array_size: header.array_size(),
        faces: header.faces(),
        cubemap: header.is_cubemap(),
        parts: texture.part_count().ok(),
    };
    if as_json {
        json::print(&info)?;
        return Ok(());
    }
    let lines = [
        ("Texture", info.texture.to_owned()),
        ("Format", info.format),
        ("Size", format!("{} x {}", info.width, info.height)),
        ("Depth", info.depth.to_string()),
        ("Mips", info.mip_count.to_string()),
        ("Array size", info.array_size.to_string()),
        ("Faces", info.faces.to_string()),
        (
            "Parts",
            info.parts
                .map_or_else(|| UNKNOWN.to_owned(), |parts| parts.to_string()),
        ),
    ];
    let mut out = io::stdout().lock();
    for (label, value) in lines {
        writeln!(out, "{:<12} {value}", format!("{label}:"))?;
    }
    Ok(())
}

fn merge(texture: &Texture<'_>, out: &Path, as_json: bool) -> Result<()> {
    let report = texture.merge_to_file(out)?;
    if as_json {
        json::print(&MergeJson {
            texture: texture.name(),
            output: json::path(out),
            parts: report.parts,
            size: report.size,
        })?;
    } else {
        writeln!(
            io::stdout().lock(),
            "merged {} and {} parts into {}, {} bytes",
            texture.name(),
            report.parts,
            out.display(),
            report.size
        )?;
    }
    Ok(())
}

fn png(texture: &Texture<'_>, out: &Path, mip: u32, tone_map: bool, as_json: bool) -> Result<()> {
    let format = texture.header().format();
    let high_range = format.is_high_range();
    let (output, kind, report) = if high_range && !tone_map {
        let output = out.with_extension("exr");
        let report = texture.exr_to_file(&output, mip)?;
        (output, "exr", (report.level, report.width, report.height))
    } else {
        let report = texture.png_to_file(out, mip)?;
        (
            out.to_path_buf(),
            "png",
            (report.level, report.width, report.height),
        )
    };
    let (level, width, height) = report;
    if as_json {
        json::print(&PngJson {
            texture: texture.name(),
            output: json::path(&output),
            format: kind,
            mip: level,
            width,
            height,
        })?;
        return Ok(());
    }
    let mut stdout = io::stdout().lock();
    let written = format!(
        "wrote mip {level} of {}, {width} x {height}",
        texture.name()
    );
    if kind == "exr" {
        writeln!(
            stdout,
            "{written}, as OpenEXR to {}: {format} is a high-range format; --tone-map writes an \
             8-bit PNG",
            output.display()
        )?;
    } else if high_range {
        writeln!(
            stdout,
            "{written}, tone-mapped to 8 bits, to {}",
            output.display()
        )?;
    } else {
        writeln!(stdout, "{written}, to {}", output.display())?;
    }
    Ok(())
}

/// Returns whether `error` is a texture failure caused by the command line: an Entry or file that
/// does not exist, a mip level the texture does not have, or an output that is one of the inputs.
pub(crate) fn is_invalid_input(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<ConversionError>(),
        Some(
            ConversionError::UnknownEntry(_)
                | ConversionError::NoSuchFile { .. }
                | ConversionError::OutputIsInput { .. }
                | ConversionError::Texture {
                    source: TextureError::NoSuchMip { .. },
                    ..
                }
        )
    )
}

/// Returns whether `error` is a texture failure caused by an unreadable Archive.
pub(crate) fn is_archive_error(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<ConversionError>(),
        Some(ConversionError::Archive(_))
    )
}
