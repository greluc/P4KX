use std::io::{self, BufWriter, Write};
use std::path::Path;

use anyhow::Result;
use p4kx_extraction::chunks::Chunk;
use p4kx_extraction::{Archive, ChunkSource};
use serde::Serialize;

use crate::cli_chunks::{ChunkFileSource, ChunksAction, ChunksArgs};
use crate::json;

const UNKNOWN: &str = "unknown";
const NONE: &str = "-";

#[derive(Serialize)]
struct ListJson<'a> {
    file: &'a str,
    container: &'static str,
    version: u32,
    chunks: Vec<ChunkJson>,
}

#[derive(Serialize)]
struct ChunkJson {
    index: usize,
    #[serde(rename = "type")]
    type_name: Option<&'static str>,
    type_id: u32,
    known: bool,
    version: u32,
    id: Option<u32>,
    byte_order: &'static str,
    offset: u64,
    size: u64,
}

impl ChunkJson {
    fn of(index: usize, chunk: &Chunk) -> Self {
        let chunk_type = chunk.chunk_type();
        Self {
            index,
            type_name: chunk_type.name(),
            type_id: chunk_type.id(),
            known: chunk_type.is_known(),
            version: chunk.version(),
            id: chunk.id(),
            byte_order: chunk.byte_order().name(),
            offset: chunk.offset(),
            size: chunk.size(),
        }
    }
}

#[derive(Serialize)]
struct DumpJson<'a> {
    file: &'a str,
    chunk: ChunkJson,
    out: String,
}

/// Runs `p4kx chunks list` or `p4kx chunks dump`.
pub(crate) fn run(args: &ChunksArgs, as_json: bool) -> Result<()> {
    match &args.action {
        ChunksAction::List(source) => list(&open(source)?, as_json),
        ChunksAction::Dump { source, chunk, out } => dump(&open(source)?, *chunk, out, as_json),
    }
}

fn open(source: &ChunkFileSource) -> Result<ChunkSource> {
    Ok(match &source.entry {
        Some(entry) => ChunkSource::from_archive(&Archive::open(&source.path)?, entry)?,
        None => ChunkSource::from_file(&source.path)?,
    })
}

fn list(source: &ChunkSource, as_json: bool) -> Result<()> {
    let file = source.file();
    let container = file.container();
    if as_json {
        json::print(&ListJson {
            file: source.name(),
            container: container.name(),
            version: container.version(),
            chunks: file
                .chunks()
                .iter()
                .enumerate()
                .map(|(index, chunk)| ChunkJson::of(index, chunk))
                .collect(),
        })?;
        return Ok(());
    }
    let mut out = BufWriter::new(io::stdout().lock());
    writeln!(out, "Chunk file:  {}", source.name())?;
    writeln!(out, "Container:   {container} {:#x}", container.version())?;
    writeln!(out, "Chunks:      {}", file.chunks().len())?;
    let unknown = file
        .chunks()
        .iter()
        .filter(|chunk| !chunk.chunk_type().is_known())
        .count();
    if unknown > 0 {
        writeln!(out, "Unknown:     {unknown}")?;
    }
    if !file.chunks().is_empty() {
        writeln!(out)?;
        writeln!(
            out,
            "{:>5}  {:<24}  {:>10}  {:>7}  {:>10}  {:<6}  {:>10}  {:>10}",
            "Index", "Type", "Type ID", "Version", "ID", "Order", "Offset", "Size"
        )?;
    }
    for (index, chunk) in file.chunks().iter().enumerate() {
        let chunk_type = chunk.chunk_type();
        let id = chunk
            .id()
            .map_or_else(|| NONE.to_owned(), |id| id.to_string());
        writeln!(
            out,
            "{index:>5}  {:<24}  {:>10}  {:>#7x}  {id:>10}  {:<6}  {:>10}  {:>10}",
            chunk_type.name().unwrap_or(UNKNOWN),
            chunk_type.hex_id(),
            chunk.version(),
            chunk.byte_order(),
            chunk.offset(),
            chunk.size()
        )?;
    }
    out.flush()?;
    Ok(())
}

fn dump(source: &ChunkSource, index: usize, out: &Path, as_json: bool) -> Result<()> {
    let chunk = source.write_chunk_to_file(index, out)?;
    if as_json {
        json::print(&DumpJson {
            file: source.name(),
            chunk: ChunkJson::of(index, &chunk),
            out: json::path(out),
        })?;
    } else {
        writeln!(
            io::stdout().lock(),
            "wrote chunk {index} ({}, {} bytes) of {} to {}",
            chunk.chunk_type().name().unwrap_or(UNKNOWN),
            chunk.size(),
            source.name(),
            out.display()
        )?;
    }
    Ok(())
}
