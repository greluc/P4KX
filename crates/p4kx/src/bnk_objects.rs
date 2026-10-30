// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::{Bank, HircKind, HircLayout, HircObject};
use serde::Serialize;

use crate::bnk::open;
use crate::bnk_objects_text::write_object;
use crate::bnk_objects_view::ObjectView;
use crate::cli::BnkObjectsArgs;
use crate::cli_bnk_objects::ObjectType;
use crate::{exit, json};

#[derive(Serialize)]
struct ObjectsJson<'a> {
    soundbank: &'a str,
    version: u32,
    bank_id: u32,
    total: usize,
    layouts: LayoutCounts,
    matched: usize,
    offset: usize,
    limit: usize,
    next_offset: Option<usize>,
    objects: Vec<ObjectView<'a>>,
}

/// How many objects of the whole bank the reader decodes, does not decode, does not support in
/// the bank's version or finds malformed.
#[derive(Default, Serialize)]
struct LayoutCounts {
    decoded: usize,
    undecoded: usize,
    unsupported: usize,
    malformed: usize,
}

impl LayoutCounts {
    fn of(objects: &[HircObject]) -> Self {
        let mut counts = Self::default();
        for object in objects {
            let count = match object.layout() {
                HircLayout::Decoded => &mut counts.decoded,
                HircLayout::Undecoded => &mut counts.undecoded,
                HircLayout::Unsupported => &mut counts.unsupported,
                HircLayout::Malformed => &mut counts.malformed,
            };
            *count = count.saturating_add(1);
        }
        counts
    }
}

/// One page of the objects that pass the filters.
struct Page<'a> {
    matched: usize,
    offset: usize,
    shown: Vec<ObjectView<'a>>,
}

impl Page<'_> {
    fn next_offset(&self) -> Option<usize> {
        let next = self.offset.saturating_add(self.shown.len());
        (next < self.matched).then_some(next)
    }
}

/// Runs `p4kx bnk objects`.
pub(crate) fn run(args: &BnkObjectsArgs, as_json: bool) -> Result<ExitCode> {
    let bank = open(&args.source)?;
    let objects = bank.soundbank().objects();
    let offset = usize::try_from(args.offset)?;
    let limit = usize::try_from(args.limit)?;
    let page = select(
        objects,
        args.kind.map(ObjectType::kind),
        args.id,
        offset,
        limit,
    );
    let counts = LayoutCounts::of(objects);
    if as_json {
        json::print(&ObjectsJson {
            soundbank: bank.name(),
            version: bank.soundbank().header().version,
            bank_id: bank.soundbank().header().id,
            total: objects.len(),
            layouts: counts,
            matched: page.matched,
            offset,
            limit,
            next_offset: page.next_offset(),
            objects: page.shown,
        })?;
        return Ok(ExitCode::from(exit::SUCCESS));
    }
    match print_text(&bank, &counts, &page) {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(ExitCode::from(exit::SUCCESS)),
    }
}

fn select(
    objects: &[HircObject],
    kind: Option<HircKind>,
    id: Option<u32>,
    offset: usize,
    limit: usize,
) -> Page<'_> {
    let mut matched = 0_usize;
    let mut shown = Vec::new();
    let chosen = objects.iter().enumerate().filter(|(_, object)| {
        kind.is_none_or(|kind| object.kind == kind) && id.is_none_or(|id| object.id == id)
    });
    for (index, object) in chosen {
        if matched >= offset && shown.len() < limit {
            shown.push(ObjectView::new(index, object));
        }
        matched = matched.saturating_add(1);
    }
    Page {
        matched,
        offset,
        shown,
    }
}

fn print_text(bank: &Bank, counts: &LayoutCounts, page: &Page<'_>) -> io::Result<()> {
    let soundbank = bank.soundbank();
    let mut out = BufWriter::new(io::stdout().lock());
    writeln!(out, "Soundbank:   {}", bank.name())?;
    writeln!(out, "Version:     {}", soundbank.header().version)?;
    writeln!(out, "Bank ID:     {}", soundbank.header().id)?;
    if !page.shown.is_empty() {
        writeln!(out)?;
    }
    for object in &page.shown {
        write_object(&mut out, object)?;
    }
    writeln!(out, "{}", summary(soundbank.objects().len(), counts, page))?;
    out.flush()
}

fn summary(total: usize, counts: &LayoutCounts, page: &Page<'_>) -> String {
    let shown = if page.shown.is_empty() {
        format!("showing none of {}", page.matched)
    } else {
        format!(
            "showing {}-{} of {}",
            page.offset.saturating_add(1),
            page.offset.saturating_add(page.shown.len()),
            page.matched
        )
    };
    let next = page.next_offset().map_or_else(String::new, |offset| {
        format!("; next page: --offset {offset}")
    });
    format!(
        "{total} objects ({} decoded, {} undecoded, {} unsupported, {} malformed); {} match: \
         {shown}{next}",
        counts.decoded, counts.undecoded, counts.unsupported, counts.malformed, page.matched
    )
}
