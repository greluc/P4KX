use crate::GfxError;
use crate::geometry::Rect;
use crate::reader::TagReader;
use crate::shape::{RecordBudget, ShapeRecord, read_records};
use crate::style::ShapeVersion;
use crate::tag::Tag;

const HAS_LAYOUT: u8 = 0x80;
const SMALL_TEXT: u8 = 0x20;
const WIDE_OFFSETS: u8 = 0x08;
const WIDE_CODES: u8 = 0x04;
const ITALIC: u8 = 0x02;
const BOLD: u8 = 0x01;
const GLYPH_STYLE: ShapeVersion = ShapeVersion(1);

/// The tag a font is defined by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontFormat {
    /// `DefineFont`: glyph shapes only; `DefineFontInfo` adds the name and codes.
    DefineFont,
    /// `DefineFont2`.
    DefineFont2,
    /// `DefineFont3`, whose glyphs are 20 times finer.
    DefineFont3,
    /// GFx `DefineCompactedFont`, whose glyph shapes are not read.
    Compacted,
    /// `DefineFont4`, an embedded CFF font whose glyphs are not read.
    DefineFont4,
}

/// A font a movie defines under an ID.
#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    /// The ID of the font.
    pub id: u16,
    /// The tag the font is defined by.
    pub format: FontFormat,
    /// The name.
    pub name: String,
    /// The full name of a `DefineFontName` tag.
    pub full_name: Option<String>,
    /// Whether the font is bold.
    pub bold: bool,
    /// Whether the font is italic.
    pub italic: bool,
    /// Whether the glyphs are aligned to pixels for small text.
    pub small_text: bool,
    /// The number of glyphs, also of a compacted font whose glyphs are not read.
    pub glyph_count: u32,
    /// The glyphs in the order the font stores them.
    pub glyphs: Vec<Glyph>,
    /// The ascent, descent, leading and kerning.
    pub layout: Option<FontLayout>,
}

/// One glyph of a font, in the font's EM square of 1024 units (20480 for `DefineFont3`).
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    /// The UTF-16 code unit (or for an ANSI font the byte) it draws.
    pub code: Option<u16>,
    /// Its outline.
    pub records: Vec<ShapeRecord>,
    /// The horizontal advance.
    pub advance: Option<i16>,
    /// The bounds.
    pub bounds: Option<Rect>,
}

/// The layout of a font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontLayout {
    /// The ascent.
    pub ascent: i32,
    /// The descent.
    pub descent: i32,
    /// The leading.
    pub leading: i32,
    /// The kerning pairs.
    pub kerning: Vec<Kerning>,
}

/// One kerning pair of a font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Kerning {
    /// The code of the left glyph.
    pub left: u16,
    /// The code of the right glyph.
    pub right: u16,
    /// The change of the advance.
    pub adjustment: i16,
}

const fn glyph(records: Vec<ShapeRecord>) -> Glyph {
    Glyph {
        code: None,
        records,
        advance: None,
        bounds: None,
    }
}

fn read_glyph(
    table: &[u8],
    base: usize,
    range: (usize, usize),
    budget: &mut RecordBudget,
) -> Result<Glyph, GfxError> {
    let (start, end) = range;
    let bytes = table
        .get(start..end.max(start))
        .ok_or(GfxError::GlyphOffset { offset: start })?;
    if bytes.is_empty() {
        return Ok(glyph(Vec::new()));
    }
    let mut reader = TagReader::new(bytes, base + start);
    Ok(glyph(read_records(&mut reader, GLYPH_STYLE, budget)?))
}

/// Reads `DefineFont`.
pub(crate) fn read_font1(tag: &Tag<'_>, budget: &mut RecordBudget) -> Result<Font, GfxError> {
    let mut reader = tag.reader();
    let id = reader.u16()?;
    let table_offset = reader.offset();
    let table = reader.rest();
    let mut offsets = TagReader::new(table, table_offset);
    let first = if table.is_empty() {
        0
    } else {
        usize::from(offsets.u16()?)
    };
    let count = first / 2;
    let mut starts = Vec::with_capacity(count);
    if count > 0 {
        starts.push(first);
    }
    for _ in 1..count {
        starts.push(usize::from(offsets.u16()?));
    }
    let mut glyphs = Vec::with_capacity(count);
    for (index, &start) in starts.iter().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(table.len());
        glyphs.push(read_glyph(table, table_offset, (start, end), budget)?);
    }
    Ok(Font {
        id,
        format: FontFormat::DefineFont,
        name: String::new(),
        full_name: None,
        bold: false,
        italic: false,
        small_text: false,
        glyph_count: u32::try_from(count).unwrap_or(u32::MAX),
        glyphs,
        layout: None,
    })
}

/// Reads `DefineFont2` or `DefineFont3`.
pub(crate) fn read_font2(
    tag: &Tag<'_>,
    format: FontFormat,
    budget: &mut RecordBudget,
) -> Result<Font, GfxError> {
    let mut reader = tag.reader();
    let id = reader.u16()?;
    let flags = reader.u8()?;
    reader.u8()?;
    let name = reader.short_string()?;
    let count = usize::from(reader.u16()?);
    let table_offset = reader.offset();
    let table = tag
        .body
        .get(table_offset - tag.offset..)
        .unwrap_or_default();
    let (starts, code_table) = read_offsets(&mut reader, count, flags & WIDE_OFFSETS != 0)?;
    let mut glyphs = Vec::with_capacity(count.min(table.len()));
    match starts {
        Some(starts) => {
            for (index, &start) in starts.iter().enumerate() {
                let end = starts.get(index + 1).copied().unwrap_or(code_table);
                glyphs.push(read_glyph(table, table_offset, (start, end), budget)?);
            }
        }
        None => glyphs.resize_with(count.min(table.len()), || glyph(Vec::new())),
    }
    let font = Font {
        id,
        format,
        name,
        full_name: None,
        bold: flags & BOLD != 0,
        italic: flags & ITALIC != 0,
        small_text: flags & SMALL_TEXT != 0,
        glyph_count: u32::try_from(count).unwrap_or(u32::MAX),
        glyphs,
        layout: None,
    };
    if count == 0 && table.len() <= code_table {
        return Ok(font);
    }
    read_codes_and_layout(font, flags, table, (table_offset, code_table))
}

/// Reads the glyph offsets and the code table offset, all relative to the offset table.
///
/// The GFx exporter strips the glyph shapes of fonts it renders from a texture and writes one
/// offset and the code table offset instead of the table; for such a font the offsets are
/// `None`.
fn read_offsets(
    reader: &mut TagReader<'_>,
    count: usize,
    wide: bool,
) -> Result<(Option<Vec<usize>>, usize), GfxError> {
    let size = if wide { 4 } else { 2 };
    let mut read_offset = || -> Result<usize, GfxError> {
        if wide {
            Ok(usize::try_from(reader.u32()?).unwrap_or(usize::MAX))
        } else {
            Ok(usize::from(reader.u16()?))
        }
    };
    if count == 0 {
        return Ok((Some(Vec::new()), read_offset().unwrap_or(0)));
    }
    let first = read_offset()?;
    if first != (count + 1) * size {
        return Ok((None, read_offset()?));
    }
    let mut starts = Vec::with_capacity(count);
    starts.push(first);
    for _ in 1..count {
        starts.push(read_offset()?);
    }
    Ok((Some(starts), read_offset()?))
}

fn read_codes_and_layout(
    mut font: Font,
    flags: u8,
    table: &[u8],
    (table_offset, code_table): (usize, usize),
) -> Result<Font, GfxError> {
    let mut reader = TagReader::new(
        table
            .get(code_table..)
            .ok_or(GfxError::GlyphOffset { offset: code_table })?,
        table_offset + code_table,
    );
    let wide_codes = flags & WIDE_CODES != 0;
    let read_code = |reader: &mut TagReader<'_>| -> Result<u16, GfxError> {
        if wide_codes {
            reader.u16()
        } else {
            reader.u8().map(u16::from)
        }
    };
    for glyph in &mut font.glyphs {
        glyph.code = Some(read_code(&mut reader)?);
    }
    if flags & HAS_LAYOUT != 0 {
        font.layout = Some(read_layout(&mut reader, &mut font.glyphs, read_code)?);
    }
    Ok(font)
}

fn read_layout(
    reader: &mut TagReader<'_>,
    glyphs: &mut [Glyph],
    read_code: impl Fn(&mut TagReader<'_>) -> Result<u16, GfxError>,
) -> Result<FontLayout, GfxError> {
    let ascent = i32::from(reader.u16()?);
    let descent = i32::from(reader.u16()?);
    let leading = i32::from(reader.i16()?);
    for glyph in glyphs.iter_mut() {
        glyph.advance = Some(reader.i16()?);
    }
    for glyph in glyphs.iter_mut() {
        glyph.bounds = Some(reader.rect()?);
    }
    let count = usize::from(reader.u16()?);
    let mut kerning = Vec::with_capacity(count.min(reader.remaining()));
    for _ in 0..count {
        kerning.push(Kerning {
            left: read_code(reader)?,
            right: read_code(reader)?,
            adjustment: reader.i16()?,
        });
    }
    Ok(FontLayout {
        ascent,
        descent,
        leading,
        kerning,
    })
}

/// Reads the ID, name and layout of the first font of a GFx `DefineCompactedFont`.
pub(crate) fn read_compacted_font(tag: &Tag<'_>) -> Result<Font, GfxError> {
    let mut reader = tag.reader();
    let id = reader.u16()?;
    let name = reader.c_string()?.into_owned();
    reader.u16()?;
    reader.u16()?;
    let ascent = i32::from(reader.i16()?);
    let descent = i32::from(reader.i16()?);
    let leading = i32::from(reader.i16()?);
    let glyph_count = reader.u32()?;
    Ok(Font {
        id,
        format: FontFormat::Compacted,
        name,
        full_name: None,
        bold: false,
        italic: false,
        small_text: false,
        glyph_count,
        glyphs: Vec::new(),
        layout: Some(FontLayout {
            ascent,
            descent,
            leading,
            kerning: Vec::new(),
        }),
    })
}
