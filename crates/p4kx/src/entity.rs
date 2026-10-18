use std::io::{self, BufWriter, Write};

use anyhow::Result;
use p4kx_assembly::{
    ArchiveFiles, Attachment, EntityNode, LoadoutResolver, PortDefinition, PortNode, PortSource,
};
use p4kx_extraction::{Archive, DataForgeEntry};

use crate::cli::{EntityAction, EntityArgs, EntityLoadoutArgs};
use crate::entity_json::{Counts, LoadoutJson, NodeJson};
use crate::json;
use crate::query_table::one_line;

/// Runs a `p4kx entity` subcommand.
pub(crate) fn run(args: &EntityArgs, as_json: bool) -> Result<()> {
    match &args.action {
        EntityAction::Loadout(args) => loadout(args, as_json),
    }
}

/// Prints the Loadout tree of an Entity.
fn loadout(args: &EntityLoadoutArgs, as_json: bool) -> Result<()> {
    let archive = Archive::open(&args.archive)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let entity = resolver.entities().find(&args.entity)?;
    let tree = resolver.resolve(&entity)?;
    let counts = Counts::of(&tree);
    if as_json {
        json::print(&LoadoutJson {
            archive: json::path(archive.path()),
            entry: entry.path(),
            entity: NodeJson::from(&tree),
            counts,
        })?;
        return Ok(());
    }
    let printed = print_tree(&tree, counts);
    match printed {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(()),
    }
}

fn print_tree(tree: &EntityNode, counts: Counts) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout().lock());
    let header = format!(
        "{}  {}  {}",
        tree.class_name(),
        tree.guid(),
        tree.record_path()
    );
    writeln!(out, "{}", one_line(&header))?;
    print_node(&mut out, tree, 1)?;
    writeln!(
        out,
        "{} ports: {} attached, {} empty, {} unresolved",
        counts.ports, counts.attached, counts.empty, counts.unresolved
    )?;
    out.flush()
}

fn print_node(out: &mut impl Write, node: &EntityNode, depth: usize) -> io::Result<()> {
    let indent = "  ".repeat(depth);
    for issue in node.issues() {
        writeln!(out, "{indent}! {}", one_line(&issue.to_string()))?;
    }
    for port in node.ports() {
        writeln!(out, "{indent}{}", one_line(&port_line(port)))?;
        if let Attachment::Entity(child) = port.attachment() {
            print_node(out, child, depth.saturating_add(1))?;
        }
    }
    Ok(())
}

fn port_line(port: &PortNode) -> String {
    let held = match port.attachment() {
        Attachment::Empty => "-".to_owned(),
        Attachment::Entity(child) => child.class_name().to_owned(),
        Attachment::Unresolved(unresolved) => format!("unresolved: {}", unresolved.error()),
    };
    let mut line = format!("{}: {held}", port.name());
    match port.source() {
        PortSource::Loadout => {}
        PortSource::Override => line.push_str(" (override)"),
        PortSource::PortDefault => line.push_str(" (port default)"),
        PortSource::Unset => line.push_str(" (unset)"),
    }
    if let Some(definition) = port.definition() {
        line.push_str(&definition_text(definition));
    }
    line
}

fn definition_text(definition: &PortDefinition) -> String {
    let mut parts = Vec::new();
    match (definition.min_size(), definition.max_size()) {
        (Some(min), Some(max)) if min == max => parts.push(format!("size {min}")),
        (Some(min), Some(max)) => parts.push(format!("size {min}-{max}")),
        _ => {}
    }
    if let Some(transform) = definition.transform()
        && !transform.helper().is_empty()
    {
        parts.push(format!("helper {}", transform.helper()));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" [{}]", parts.join(", "))
    }
}
