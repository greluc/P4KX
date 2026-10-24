// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};
use std::path::Path;

use p4kx_extraction::{Tag, TagTreeIssues};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

use crate::dcb_tags::Listing;
use crate::json;

/// One tag of the flat list, with its parent and children by GUID.
struct Flat<'t>(Tag<'t>);

/// One tag of the tree, with its children nested.
struct Nested<'t>(Tag<'t>);

struct Issues(TagTreeIssues);

struct Tags<'a, 't> {
    listing: &'a Listing<'t>,
    tree: bool,
}

fn head<S: SerializeMap>(map: &mut S, tag: &Tag<'_>) -> Result<(), S::Error> {
    map.serialize_entry("guid", &tag.guid().to_string())?;
    map.serialize_entry("name", tag.name())?;
    map.serialize_entry("path", tag.path())?;
    map.serialize_entry("legacy_id", &tag.legacy_id())
}

impl Serialize for Flat<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let tag = &self.0;
        let mut map = serializer.serialize_map(Some(7))?;
        head(&mut map, tag)?;
        map.serialize_entry(
            "parent",
            &tag.parent().map(|parent| parent.guid().to_string()),
        )?;
        map.serialize_entry("depth", &tag.depth())?;
        let children: Vec<String> = tag
            .children()
            .map(|child| child.guid().to_string())
            .collect();
        map.serialize_entry("children", &children)?;
        map.end()
    }
}

impl Serialize for Nested<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let tag = &self.0;
        let mut map = serializer.serialize_map(Some(5))?;
        head(&mut map, tag)?;
        let children: Vec<Nested<'_>> = tag.children().map(Nested).collect();
        map.serialize_entry("children", &children)?;
        map.end()
    }
}

impl Serialize for Issues {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(7))?;
        map.serialize_entry("unreadable", &self.0.unreadable())?;
        map.serialize_entry("dangling", &self.0.dangling())?;
        map.serialize_entry("repeated", &self.0.repeated())?;
        map.serialize_entry("too_deep", &self.0.too_deep())?;
        map.serialize_entry("too_long", &self.0.too_long())?;
        map.serialize_entry("duplicate", &self.0.duplicate())?;
        map.serialize_entry("unlisted", &self.0.unlisted())?;
        map.end()
    }
}

impl Serialize for Tags<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.tree {
            let mut seq = serializer.serialize_seq(Some(self.listing.tops.len()))?;
            for top in &self.listing.tops {
                seq.serialize_element(&Nested(*top))?;
            }
            seq.end()
        } else {
            let mut seq = serializer.serialize_seq(Some(self.listing.rows.len()))?;
            for row in &self.listing.rows {
                seq.serialize_element(&Flat(row.tag))?;
            }
            seq.end()
        }
    }
}

struct Document<'a, 't> {
    archive: String,
    listing: &'a Listing<'t>,
    tree: bool,
}

impl Serialize for Document<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("archive", &self.archive)?;
        map.serialize_entry("tag_count", &self.listing.total)?;
        map.serialize_entry("shown_count", &self.listing.rows.len())?;
        map.serialize_entry("issues", &Issues(self.listing.issues))?;
        map.serialize_entry("tree", &self.tree)?;
        map.serialize_entry(
            "tags",
            &Tags {
                listing: self.listing,
                tree: self.tree,
            },
        )?;
        map.end()
    }
}

/// Writes the listing as one line of JSON: the tags flat in tree order, or with `tree` nested.
pub(crate) fn write(
    out: &mut impl Write,
    archive: &Path,
    listing: &Listing<'_>,
    tree: bool,
) -> io::Result<()> {
    let document = Document {
        archive: json::path(archive),
        listing,
        tree,
    };
    serde_json::to_writer(&mut *out, &document)?;
    writeln!(out)
}
