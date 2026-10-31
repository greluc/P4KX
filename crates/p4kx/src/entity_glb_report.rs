// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The text report of `p4kx entity glb`, and the texture and issue lines that `armour glb` and
//! `chf glb` print the same way.

use std::io::{self, BufWriter, Write};
use std::path::Path;

use p4kx_assembly::{Assembly, EntityNode, Issue, LoadoutChanges};
use p4kx_extraction::GlbSummary;

use crate::entity_glb_body::print_body;
use crate::entity_glb_placements::painted;
use crate::query_table::one_line;

fn print_changes(out: &mut impl Write, changes: &LoadoutChanges) -> io::Result<()> {
    for port in changes.exclusions() {
        writeln!(out, "excluded {}", one_line(port))?;
    }
    for (port, entity) in changes.replacements() {
        writeln!(out, "replaced {} by {}", one_line(port), one_line(entity))?;
    }
    Ok(())
}

fn print_hidden(out: &mut impl Write, assembly: &Assembly) -> io::Result<()> {
    match assembly.hidden_parts() {
        0 => Ok(()),
        hidden => writeln!(out, "{hidden} parts hidden on ports flagged invisible"),
    }
}

/// Prints the embedded textures and the materials that fall back to their plain colour.
pub(crate) fn print_textures(out: &mut impl Write, summary: &GlbSummary) -> io::Result<()> {
    if summary.textures > 0 || !summary.texture_fallbacks.is_empty() {
        writeln!(
            out,
            "embedded {} images of {} pixels in {} of {} materials",
            summary.textures, summary.texture_pixels, summary.textured_materials, summary.materials
        )?;
    }
    for fallback in &summary.texture_fallbacks {
        writeln!(
            out,
            "no {} texture {} for material {}: {}; the plain colour is used",
            fallback.role,
            one_line(&fallback.file),
            one_line(&fallback.material),
            one_line(&fallback.reason)
        )?;
    }
    Ok(())
}

/// Prints the summary of one run to stdout.
pub(crate) fn print_report(
    assembly: &Assembly,
    summary: &GlbSummary,
    out: &Path,
    tree: &EntityNode,
    changes: &LoadoutChanges,
) -> io::Result<()> {
    let mut stdout = BufWriter::new(io::stdout().lock());
    writeln!(
        stdout,
        "wrote {} of {}: {} parts ({} with models), {} nodes, {} meshes, {} primitives, {} materials, {} vertices, {} triangles, {} bytes",
        out.display(),
        one_line(tree.class_name()),
        assembly.parts().len(),
        assembly.modelled_parts(),
        summary.nodes,
        summary.meshes,
        summary.primitives,
        summary.materials,
        summary.vertices,
        summary.triangles,
        summary.bytes
    )?;
    print_changes(&mut stdout, changes)?;
    print_body(&mut stdout, tree)?;
    print_hidden(&mut stdout, assembly)?;
    if let Some(interior) = assembly.interior() {
        writeln!(
            stdout,
            "Interior: {} object containers, {} parts ({} models, {} with geometry, {} lights)",
            interior.containers().len(),
            interior.parts(),
            interior.model_parts(),
            interior.modelled_parts(),
            interior.lights()
        )?;
    }
    let painted = painted(assembly);
    if painted > 0 {
        writeln!(stdout, "{painted} parts drawn with a tint palette")?;
    }
    print_textures(&mut stdout, summary)?;
    if summary.lod_fallbacks > 0 {
        writeln!(
            stdout,
            "{} models lack LOD{} and are written at the nearest lower level",
            summary.lod_fallbacks,
            assembly.lod()
        )?;
    }
    if summary.shared_meshes + summary.shared_materials + summary.shared_textures > 0 {
        writeln!(
            stdout,
            "shared {} meshes, {} materials and {} textures instead of writing them again",
            summary.shared_meshes, summary.shared_materials, summary.shared_textures
        )?;
    }
    if summary.hidden_primitives > 0 {
        writeln!(
            stdout,
            "left out {} submeshes whose material draws nothing",
            summary.hidden_primitives
        )?;
    }
    print_issues(&mut stdout, assembly.issues())?;
    stdout.flush()
}

/// Prints one line per issue: `skipped` for a part that is left out, `note` for the others.
pub(crate) fn print_issues(out: &mut impl Write, issues: &[Issue]) -> io::Result<()> {
    for issue in issues {
        let what = if issue.kind().skips() {
            "skipped"
        } else {
            "note"
        };
        let path = if issue.path().is_empty() {
            issue.name().to_owned()
        } else {
            format!("{} ({})", issue.path(), issue.name())
        };
        writeln!(
            out,
            "{what} {}: {}",
            one_line(&path),
            one_line(&issue.kind().to_string())
        )?;
    }
    Ok(())
}
