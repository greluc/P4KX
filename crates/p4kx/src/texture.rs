use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;
use p4kx_extraction::{Archive, ConversionError, CubemapLayout, Texture, TextureError};
use serde::Serialize;

use crate::cli::{TextureAction, TextureArgs, TextureSource};
use crate::json;

const UNKNOWN: &str = "unknown";
const NONE: &str = "none";

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
    alpha_tail: Option<AlphaTailJson>,
}

#[derive(Serialize)]
struct AlphaTailJson {
    format: String,
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
    files: Vec<String>,
    mip: u32,
    width: u32,
    height: u32,
    alpha_tail: bool,
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
            cross,
        } => {
            let layout = if *cross {
                CubemapLayout::Cross
            } else {
                CubemapLayout::Faces
            };
            with_texture(source, |texture| png(texture, out, *mip, layout, as_json))
        }
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
        alpha_tail: texture.alpha_tail().map(|tail| AlphaTailJson {
            format: tail.header().format().to_string(),
            parts: tail.part_count().ok(),
        }),
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
        ("Parts", count(info.parts)),
        (
            "Alpha tail",
            info.alpha_tail.map_or_else(
                || NONE.to_owned(),
                |tail| format!("{}, {} parts", tail.format, count(tail.parts)),
            ),
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

fn png(
    texture: &Texture<'_>,
    out: &Path,
    mip: u32,
    layout: CubemapLayout,
    as_json: bool,
) -> Result<()> {
    let report = texture.png_to_file_as(out, mip, layout)?;
    if as_json {
        json::print(&PngJson {
            texture: texture.name(),
            output: json::path(out),
            files: report.files.iter().map(|file| json::path(file)).collect(),
            mip: report.level,
            width: report.width,
            height: report.height,
            alpha_tail: report.alpha_tail,
        })?;
    } else {
        let files: Vec<String> = report
            .files
            .iter()
            .map(|file| file.display().to_string())
            .collect();
        writeln!(
            io::stdout().lock(),
            "wrote mip {} of {}{}, {} x {}, to {}",
            report.level,
            texture.name(),
            if report.alpha_tail {
                " with its alpha tail"
            } else {
                ""
            },
            report.width,
            report.height,
            files.join(", ")
        )?;
    }
    Ok(())
}

fn count(parts: Option<u32>) -> String {
    parts.map_or_else(|| UNKNOWN.to_owned(), |parts| parts.to_string())
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
