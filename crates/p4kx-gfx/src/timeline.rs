use std::collections::BTreeMap;

use crate::GfxError;
use crate::place::{PlaceCommand, Placement};
use crate::tag::Tag;

const PLACE_HAS_CHARACTER: u8 = 0x02;
const PLACE3_HAS_CLASS_NAME: u8 = 0x08;
const PLACE3_HAS_IMAGE: u8 = 0x10;

/// A sprite: a movie clip with its own timeline, defined under an ID.
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    /// The ID of the sprite.
    pub id: u16,
    /// The frames its header declares.
    pub frame_count: u16,
    /// What its timeline holds.
    pub timeline: Timeline,
}

/// What a timeline holds: its frames, labels and the definitions it places.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Timeline {
    /// The `ShowFrame` tags.
    pub frames: u32,
    /// The frame labels in order.
    pub labels: Vec<String>,
    /// The IDs of the definitions placed on it, each once, in ascending order.
    pub placed: Vec<u16>,
    /// The display list of the first frame in ascending depth order.
    pub first_frame: Vec<Placement>,
    /// The tags, End included.
    pub tag_count: u32,
}

/// The display list a timeline builds up to the end of its first frame.
#[derive(Debug, Default)]
pub(crate) struct DisplayList(BTreeMap<u16, Placement>);

impl DisplayList {
    /// Applies a `PlaceObject` tag.
    pub(crate) fn place(&mut self, command: PlaceCommand) {
        let depth = command.depth;
        if let Some(placement) = command.apply(self.0.remove(&depth)) {
            self.0.insert(depth, placement);
        }
    }

    /// Applies a `RemoveObject` or `RemoveObject2` tag.
    pub(crate) fn remove(&mut self, tag: &Tag<'_>) -> Result<(), GfxError> {
        let mut reader = tag.reader();
        if tag.code == 5 {
            reader.u16()?;
        }
        self.0.remove(&reader.u16()?);
        Ok(())
    }
}

impl Timeline {
    /// Sorts the placed IDs, removes repeats and keeps the first frame once the timeline is read.
    pub(crate) fn finish(&mut self, display_list: DisplayList) {
        self.placed.sort_unstable();
        self.placed.dedup();
        self.first_frame = display_list.0.into_values().collect();
    }
}

/// Returns the ID of the definition a `PlaceObject`, `PlaceObject2` or `PlaceObject3` tag places, if
/// it places one rather than changing a placement.
pub(crate) fn placed_id(tag: &Tag<'_>) -> Result<Option<u16>, GfxError> {
    let mut reader = tag.reader();
    match tag.code {
        4 => reader.u16().map(Some),
        26 => {
            let flags = reader.u8()?;
            reader.u16()?;
            if flags & PLACE_HAS_CHARACTER == 0 {
                return Ok(None);
            }
            reader.u16().map(Some)
        }
        _ => {
            let flags = reader.u8()?;
            let flags3 = reader.u8()?;
            reader.u16()?;
            let has_character = flags & PLACE_HAS_CHARACTER != 0;
            if flags3 & PLACE3_HAS_CLASS_NAME != 0
                || (flags3 & PLACE3_HAS_IMAGE != 0 && has_character)
            {
                reader.c_string()?;
            }
            if !has_character {
                return Ok(None);
            }
            reader.u16().map(Some)
        }
    }
}

/// A name a movie exports a definition under, of an `ExportAssets` tag.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Export {
    /// The ID of the definition.
    pub id: u16,
    /// The name.
    pub name: String,
}

/// Reads the names of an `ExportAssets` tag.
pub(crate) fn read_exports(tag: &Tag<'_>) -> Result<Vec<Export>, GfxError> {
    let mut reader = tag.reader();
    let count = usize::from(reader.u16()?);
    let mut exports = Vec::with_capacity(count.min(reader.remaining()));
    for _ in 0..count {
        exports.push(Export {
            id: reader.u16()?,
            name: reader.c_string()?.into_owned(),
        });
    }
    Ok(exports)
}

/// What the GFx exporter states about a movie, of an `ExporterInfo` tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExporterInfo {
    /// The exporter version.
    pub version: u16,
    /// The export flags, from version 0x10A on.
    pub flags: u32,
    /// The format the exporter wrote the bitmaps in, as GFx numbers file formats.
    pub bitmap_format: u16,
    /// The prefix of the exported file names.
    pub prefix: String,
    /// The name of the SWF the movie was exported from.
    pub swf_name: String,
}

/// Reads `ExporterInfo`.
pub(crate) fn read_exporter_info(tag: &Tag<'_>) -> Result<ExporterInfo, GfxError> {
    let mut reader = tag.reader();
    let version = reader.u16()?;
    let flags = if version >= 0x10A { reader.u32()? } else { 0 };
    Ok(ExporterInfo {
        version,
        flags,
        bitmap_format: reader.u16()?,
        prefix: reader.short_string()?,
        swf_name: reader.short_string()?,
    })
}
