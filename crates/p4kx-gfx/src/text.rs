use std::borrow::Cow;

use crate::GfxError;
use crate::geometry::{Matrix, Rect, Rgba};
use crate::tag::Tag;

/// The most glyph entries all static texts of a movie may hold together.
pub const MAX_TEXT_GLYPHS: usize = 1 << 20;

const RECORD_TYPE: u8 = 0x80;
const RECORD_HAS_FONT: u8 = 0x08;
const RECORD_HAS_COLOR: u8 = 0x04;
const RECORD_HAS_Y: u8 = 0x02;
const RECORD_HAS_X: u8 = 0x01;

const EDIT_HAS_TEXT: u8 = 0x80;
const EDIT_WORD_WRAP: u8 = 0x40;
const EDIT_MULTILINE: u8 = 0x20;
const EDIT_PASSWORD: u8 = 0x10;
const EDIT_HAS_COLOR: u8 = 0x04;
const EDIT_HAS_MAX_LENGTH: u8 = 0x02;
const EDIT_HAS_FONT: u8 = 0x01;
const EDIT_HAS_CLASS: u8 = 0x80;
const EDIT_AUTO_SIZE: u8 = 0x40;
const EDIT_HAS_LAYOUT: u8 = 0x20;
const EDIT_HTML: u8 = 0x02;

/// A `DefineText` or `DefineText2` tag: glyphs placed by index, in twips.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticText {
    /// The ID of the text.
    pub id: u16,
    /// The bounds.
    pub bounds: Rect,
    /// Maps the text into its timeline.
    pub matrix: Matrix,
    /// The records in order; each starts from the state the one before leaves.
    pub records: Vec<TextRecord>,
}

/// A run of glyphs of a [`StaticText`] and what it changes of the state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRecord {
    /// The font from here on.
    pub font_id: Option<u16>,
    /// The colour from here on.
    pub color: Option<Rgba>,
    /// The new horizontal pen position in twips.
    pub x_offset: Option<i16>,
    /// The new baseline in twips.
    pub y_offset: Option<i16>,
    /// The font height in twips from here on.
    pub height: Option<u16>,
    /// The glyphs of the run.
    pub glyphs: Vec<GlyphEntry>,
}

/// One glyph of a [`TextRecord`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphEntry {
    /// The index in the glyph table of the font.
    pub index: u32,
    /// The advance in twips.
    pub advance: i32,
}

/// A `DefineEditText` tag: a text field with its text, font and layout.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the tag stores one flag per behaviour"
)]
pub struct EditText {
    /// The ID of the field.
    pub id: u16,
    /// The bounds of the field.
    pub bounds: Rect,
    /// Whether lines wrap at the width of the field.
    pub word_wrap: bool,
    /// Whether the field shows more than one line.
    pub multiline: bool,
    /// Whether the field shows asterisks instead of its text.
    pub password: bool,
    /// Whether the field resizes to its text.
    pub auto_size: bool,
    /// Whether the text is HTML of the Flash subset.
    pub html: bool,
    /// The font by ID.
    pub font_id: Option<u16>,
    /// The font by name.
    pub font_class: Option<String>,
    /// The font height in twips.
    pub height: Option<u16>,
    /// The text colour.
    pub color: Option<Rgba>,
    /// The most characters the user may type.
    pub max_length: Option<u16>,
    /// The paragraph layout.
    pub layout: Option<EditLayout>,
    /// The name of the variable the text is bound to.
    pub variable: String,
    /// The initial text.
    pub text: Option<String>,
}

/// The paragraph layout of an [`EditText`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditLayout {
    /// 0 left, 1 right, 2 centre, 3 justified.
    pub align: u8,
    /// The left margin in twips.
    pub left_margin: u16,
    /// The right margin in twips.
    pub right_margin: u16,
    /// The indent of the first line in twips.
    pub indent: u16,
    /// The extra space between lines in twips.
    pub leading: i16,
}

/// The glyph entries a movie may still hold.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GlyphBudget(usize);

impl GlyphBudget {
    /// The budget of a whole movie.
    pub(crate) const MOVIE: Self = Self(MAX_TEXT_GLYPHS);

    fn take(&mut self, count: usize) -> Result<(), GfxError> {
        self.0 = self
            .0
            .checked_sub(count)
            .ok_or(GfxError::TooManyTextGlyphs {
                limit: MAX_TEXT_GLYPHS,
            })?;
        Ok(())
    }
}

/// Reads `DefineText` or, with `alpha`, `DefineText2`.
pub(crate) fn read_static_text(
    tag: &Tag<'_>,
    alpha: bool,
    budget: &mut GlyphBudget,
) -> Result<StaticText, GfxError> {
    let mut reader = tag.reader();
    let id = reader.u16()?;
    let bounds = reader.rect()?;
    let matrix = reader.matrix()?;
    let glyph_bits = u32::from(reader.u8()?);
    let advance_bits = u32::from(reader.u8()?);
    let mut records = Vec::new();
    while reader.remaining() > 0 {
        let flags = reader.u8()?;
        if flags & RECORD_TYPE == 0 {
            break;
        }
        let font_id = (flags & RECORD_HAS_FONT != 0)
            .then(|| reader.u16())
            .transpose()?;
        let color = (flags & RECORD_HAS_COLOR != 0)
            .then(|| reader.color(alpha))
            .transpose()?;
        let x_offset = (flags & RECORD_HAS_X != 0)
            .then(|| reader.i16())
            .transpose()?;
        let y_offset = (flags & RECORD_HAS_Y != 0)
            .then(|| reader.i16())
            .transpose()?;
        let height = (flags & RECORD_HAS_FONT != 0)
            .then(|| reader.u16())
            .transpose()?;
        let count = usize::from(reader.u8()?);
        budget.take(count)?;
        let mut glyphs = Vec::with_capacity(count);
        for _ in 0..count {
            glyphs.push(GlyphEntry {
                index: reader.ub(glyph_bits)?,
                advance: reader.sb(advance_bits)?,
            });
        }
        records.push(TextRecord {
            font_id,
            color,
            x_offset,
            y_offset,
            height,
            glyphs,
        });
    }
    Ok(StaticText {
        id,
        bounds,
        matrix,
        records,
    })
}

/// Reads `DefineEditText`.
pub(crate) fn read_edit_text(tag: &Tag<'_>) -> Result<EditText, GfxError> {
    let mut reader = tag.reader();
    let id = reader.u16()?;
    let bounds = reader.rect()?;
    let first = reader.u8()?;
    let second = reader.u8()?;
    let font_id = (first & EDIT_HAS_FONT != 0)
        .then(|| reader.u16())
        .transpose()?;
    let font_class = (second & EDIT_HAS_CLASS != 0)
        .then(|| reader.c_string().map(Cow::into_owned))
        .transpose()?;
    let height = (first & EDIT_HAS_FONT != 0 || second & EDIT_HAS_CLASS != 0)
        .then(|| reader.u16())
        .transpose()?;
    let color = (first & EDIT_HAS_COLOR != 0)
        .then(|| reader.rgba())
        .transpose()?;
    let max_length = (first & EDIT_HAS_MAX_LENGTH != 0)
        .then(|| reader.u16())
        .transpose()?;
    let layout = if second & EDIT_HAS_LAYOUT != 0 {
        Some(EditLayout {
            align: reader.u8()?,
            left_margin: reader.u16()?,
            right_margin: reader.u16()?,
            indent: reader.u16()?,
            leading: reader.i16()?,
        })
    } else {
        None
    };
    let variable = reader.c_string()?.into_owned();
    let text = (first & EDIT_HAS_TEXT != 0)
        .then(|| reader.c_string().map(Cow::into_owned))
        .transpose()?;
    Ok(EditText {
        id,
        bounds,
        word_wrap: first & EDIT_WORD_WRAP != 0,
        multiline: first & EDIT_MULTILINE != 0,
        password: first & EDIT_PASSWORD != 0,
        auto_size: second & EDIT_AUTO_SIZE != 0,
        html: second & EDIT_HTML != 0,
        font_id,
        font_class,
        height,
        color,
        max_length,
        layout,
        variable,
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::GlyphBudget;

    #[test]
    fn the_budget_refuses_what_is_left_over() {
        let mut budget = GlyphBudget(3);
        assert!(budget.take(2).is_ok());
        assert!(budget.take(2).is_err());
    }
}
