use crate::GfxError;
use crate::font::Font;
use crate::tag::Tag;

const SMALL_TEXT: u8 = 0x20;
const ITALIC: u8 = 0x04;
const BOLD: u8 = 0x02;
const WIDE_CODES: u8 = 0x01;
const FONT_INFO2: u16 = 62;

/// The name, style and codes a `DefineFontInfo` or `DefineFontInfo2` tag gives a `DefineFont` font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FontInfo {
    pub(crate) id: u16,
    name: String,
    flags: u8,
    codes: Vec<u16>,
}

impl FontInfo {
    /// Gives `font` the name, style and glyph codes.
    pub(crate) fn apply(self, font: &mut Font) {
        font.name = self.name;
        font.bold = self.flags & BOLD != 0;
        font.italic = self.flags & ITALIC != 0;
        font.small_text = self.flags & SMALL_TEXT != 0;
        for (glyph, code) in font.glyphs.iter_mut().zip(self.codes) {
            glyph.code = Some(code);
        }
    }
}

/// Reads `DefineFontInfo` or `DefineFontInfo2`.
pub(crate) fn read_font_info(tag: &Tag<'_>) -> Result<FontInfo, GfxError> {
    let mut reader = tag.reader();
    let id = reader.u16()?;
    let name = reader.short_string()?;
    let flags = reader.u8()?;
    if tag.code == FONT_INFO2 {
        reader.u8()?;
    }
    let codes = reader.rest();
    let codes = if flags & WIDE_CODES != 0 || tag.code == FONT_INFO2 {
        codes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&pair| u16::from_le_bytes(pair))
            .collect()
    } else {
        codes.iter().copied().map(u16::from).collect()
    };
    Ok(FontInfo {
        id,
        name,
        flags,
        codes,
    })
}

/// Reads the ID and full name of `DefineFontName`.
pub(crate) fn read_font_name(tag: &Tag<'_>) -> Result<(u16, String), GfxError> {
    let mut reader = tag.reader();
    let id = reader.u16()?;
    Ok((id, reader.c_string()?.into_owned()))
}
