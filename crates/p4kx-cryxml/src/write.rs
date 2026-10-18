use std::io::{self, Write};

use crate::document::Document;
use crate::element::{Children, Element};
use crate::escape::{Context, write_escaped, write_name};

const DECLARATION: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
const MAX_INDENT_LEVELS: usize = 64;
const INDENT: [u8; 2 * MAX_INDENT_LEVELS] = [b' '; 2 * MAX_INDENT_LEVELS];

/// Writes `document` as indented XML without recursion; indentation stops growing at 64 levels
/// so that deep input cannot blow up the output quadratically.
pub(crate) fn write<W: Write>(document: &Document<'_>, mut out: W) -> io::Result<()> {
    out.write_all(DECLARATION)?;
    let mut open: Vec<(Element<'_, '_>, Children<'_, '_>)> = Vec::new();
    let root = document.root();
    if start(&mut out, root, 0)? {
        open.push((root, root.children()));
    }
    while let Some((element, children)) = open.last_mut() {
        if let Some(child) = children.next() {
            if start(&mut out, child, open.len())? {
                open.push((child, child.children()));
            }
        } else {
            let element = *element;
            open.pop();
            indent(&mut out, open.len())?;
            end_tag(&mut out, element)?;
        }
    }
    out.flush()
}

/// Writes the start of `element`, or all of it when it has no children; returns whether its
/// children and end tag remain to be written.
fn start(out: &mut impl Write, element: Element<'_, '_>, depth: usize) -> io::Result<bool> {
    indent(out, depth)?;
    out.write_all(b"<")?;
    write_name(out, element.name())?;
    for attribute in element.attributes() {
        out.write_all(b" ")?;
        write_name(out, attribute.name())?;
        out.write_all(b"=\"")?;
        write_escaped(out, attribute.value(), Context::Attribute)?;
        out.write_all(b"\"")?;
    }
    let has_children = element.has_children();
    match element.content() {
        None if !has_children => out.write_all(b"/>\n")?,
        None => out.write_all(b">\n")?,
        Some(content) => {
            out.write_all(b">")?;
            write_escaped(out, content, Context::Text)?;
            if has_children {
                out.write_all(b"\n")?;
            } else {
                end_tag(out, element)?;
            }
        }
    }
    Ok(has_children)
}

fn end_tag(out: &mut impl Write, element: Element<'_, '_>) -> io::Result<()> {
    out.write_all(b"</")?;
    write_name(out, element.name())?;
    out.write_all(b">\n")
}

fn indent(out: &mut impl Write, depth: usize) -> io::Result<()> {
    let width = 2 * depth.min(MAX_INDENT_LEVELS);
    out.write_all(INDENT.get(..width).unwrap_or_default())
}
