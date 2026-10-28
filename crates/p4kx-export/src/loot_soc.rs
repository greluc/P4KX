// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_extraction::Guid;
use p4kx_extraction::chunks::{ChunkFile, ChunkType, Container};
use p4kx_extraction::cryxml::{self, Xml};

use crate::starmap_xml::{Node, attribute};

/// The chunk of a `.soc` file that holds its entities as `CryXmlB`.
const ENTITIES: ChunkType = ChunkType::new(Container::CrCh, 0x0004);

/// The most entities of one `.soc` file that are counted.
const MAX_ENTITIES: usize = 1 << 20;

/// How many entities of a `.soc` file there are and how many carry each tag.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EntityTags {
    pub(crate) entities: u32,
    pub(crate) by_tag: HashMap<Guid, u32>,
}

/// Counts the entities of the `.soc` file `bytes` and the tags they carry: the `TagId` of every
/// `Tag` below the `Tags` of an `Entity`.
///
/// # Errors
///
/// Returns why the file cannot be read: it is no chunk file, a chunk holds no XML, it lists more
/// than [`MAX_ENTITIES`] entities.
pub(crate) fn tag_counts(bytes: &[u8]) -> Result<EntityTags, &'static str> {
    let file = ChunkFile::parse(bytes).map_err(|_| "its .soc is no chunk file")?;
    let mut counts = EntityTags::default();
    for (index, _) in file.chunks_of(ENTITIES) {
        let data = file.chunk_data(index).unwrap_or_default();
        count_xml(data, &mut counts)?;
    }
    Ok(counts)
}

/// Adds the entities of the XML document `bytes`, `CryXml` or text, to `counts`.
fn count_xml(bytes: &[u8], counts: &mut EntityTags) -> Result<(), &'static str> {
    match cryxml::read(bytes).map_err(|_| "its entity chunk is no XML")? {
        Xml::CryXml(document) => count_entities(document.root(), counts),
        Xml::Text(text) => {
            let text = std::str::from_utf8(text).map_err(|_| "its entity chunk is no UTF-8")?;
            cryxml::check_text_depth(text).map_err(|_| "its entity chunk nests too deep")?;
            let document =
                roxmltree::Document::parse(text).map_err(|_| "its entity chunk is no XML")?;
            count_entities(document.root_element(), counts)
        }
    }
}

fn count_entities<'d, N: Node<'d>>(root: N, counts: &mut EntityTags) -> Result<(), &'static str> {
    for entity in root
        .children()
        .filter(|child| child.name().eq_ignore_ascii_case("Entity"))
    {
        if usize::try_from(counts.entities).map_or(true, |seen| seen >= MAX_ENTITIES) {
            return Err("its .soc lists too many entities");
        }
        counts.entities += 1;
        for tags in entity
            .children()
            .filter(|child| child.name().eq_ignore_ascii_case("Tags"))
        {
            for id in tags
                .children()
                .filter(|child| child.name().eq_ignore_ascii_case("Tag"))
                .filter_map(|tag| attribute(tag, "TagId"))
                .filter_map(|id| id.trim().parse::<Guid>().ok())
            {
                *counts.by_tag.entry(id).or_default() += 1;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAG_A: &str = "00ead468-9073-485b-a6df-ca6b94053603";
    const TAG_B: &str = "59ca5e36-b9a2-4459-ad0e-5745bcf5d899";

    fn guid(text: &str) -> Guid {
        text.parse().expect("a GUID")
    }

    fn counted(xml: &str) -> EntityTags {
        let mut counts = EntityTags::default();
        count_xml(xml.as_bytes(), &mut counts).expect("counts");
        counts
    }

    #[test]
    fn each_tag_is_counted_once_per_entity_that_carries_it() {
        let counts = counted(&format!(
            r#"<Entities>
              <Entity Name="a"><Tags><Tag TagId="{TAG_A}"/><Tag TagId="{TAG_B}"/></Tags></Entity>
              <Entity Name="b"><Tags><Tag TagId="{TAG_A}"/></Tags></Entity>
              <Entity Name="c"/>
              <Other/>
            </Entities>"#
        ));
        assert_eq!(counts.entities, 3);
        assert_eq!(counts.by_tag.get(&guid(TAG_A)), Some(&2));
        assert_eq!(counts.by_tag.get(&guid(TAG_B)), Some(&1));
        assert_eq!(counts.by_tag.len(), 2);
    }

    #[test]
    fn a_tag_that_is_no_guid_is_left_out() {
        let counts =
            counted(r#"<Entities><Entity><Tags><Tag TagId="x"/></Tags></Entity></Entities>"#);
        assert_eq!(counts.entities, 1);
        assert!(counts.by_tag.is_empty());
    }

    #[test]
    fn bytes_that_are_no_chunk_file_are_an_error() {
        assert!(tag_counts(b"<Entities/>").is_err());
        assert!(tag_counts(b"").is_err());
    }
}
