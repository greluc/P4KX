use crate::gfx::{BitWriter, rect, translation};

/// One record of [`define_text_body`]: what it changes of the state and its glyph entries.
#[derive(Debug, Clone, Default)]
pub struct TestTextRecord {
    /// The font ID and its height in twips.
    pub font: Option<(u16, u16)>,
    /// The colour as RGBA.
    pub color: Option<[u8; 4]>,
    /// The new pen position in twips.
    pub x: Option<i16>,
    /// The new baseline in twips.
    pub y: Option<i16>,
    /// The glyph index and the advance in twips of each glyph.
    pub glyphs: Vec<(u32, i32)>,
}

/// Returns the body of a `DefineText` tag (or with `alpha` a `DefineText2` tag) with 8-bit glyph
/// indexes and 16-bit advances.
#[must_use]
pub fn define_text_body(
    id: u16,
    bounds: [i32; 4],
    alpha: bool,
    records: &[TestTextRecord],
) -> Vec<u8> {
    let mut bits = BitWriter::default();
    bits.bytes(&id.to_le_bytes());
    rect(&mut bits, bounds);
    translation(&mut bits, 0, 0);
    bits.bytes(&[8, 16]);
    for record in records {
        let flags = 0x80
            | u8::from(record.font.is_some()) << 3
            | u8::from(record.color.is_some()) << 2
            | u8::from(record.y.is_some()) << 1
            | u8::from(record.x.is_some());
        bits.bytes(&[flags]);
        if let Some((font, _)) = record.font {
            bits.bytes(&font.to_le_bytes());
        }
        if let Some([red, green, blue, opacity]) = record.color {
            bits.bytes(&[red, green, blue]);
            if alpha {
                bits.bytes(&[opacity]);
            }
        }
        if let Some(x) = record.x {
            bits.bytes(&x.to_le_bytes());
        }
        if let Some(y) = record.y {
            bits.bytes(&y.to_le_bytes());
        }
        if let Some((_, height)) = record.font {
            bits.bytes(&height.to_le_bytes());
        }
        bits.bytes(&[u8::try_from(record.glyphs.len()).unwrap_or(u8::MAX)]);
        for &(index, advance) in &record.glyphs {
            bits.ub(8, index);
            bits.sb(16, advance);
        }
    }
    bits.bytes(&[0]);
    bits.finish()
}

/// Writes the body of a `DefineEditText` tag.
#[derive(Debug, Clone)]
pub struct TestEditText {
    id: u16,
    bounds: [i32; 4],
    text: Option<String>,
    html: u8,
    font: Option<(u16, u16)>,
    class: Option<(String, u16)>,
    color: Option<[u8; 4]>,
    flags: u8,
    layout: Option<(u8, u16, u16, u16, i16)>,
}

impl TestEditText {
    /// Starts a field of `id` with `bounds` in twips and no text.
    #[must_use]
    pub const fn new(id: u16, bounds: [i32; 4]) -> Self {
        Self {
            id,
            bounds,
            text: None,
            html: 0,
            font: None,
            class: None,
            color: None,
            flags: 0,
            layout: None,
        }
    }

    /// Sets the initial text.
    #[must_use]
    pub fn text(mut self, text: &str) -> Self {
        self.text = Some(text.to_owned());
        self
    }

    /// Marks the text as HTML.
    #[must_use]
    pub const fn html(mut self) -> Self {
        self.html = 0x02;
        self
    }

    /// Sets the font by ID and its height in twips.
    #[must_use]
    pub const fn font(mut self, id: u16, height: u16) -> Self {
        self.font = Some((id, height));
        self
    }

    /// Sets the font by name and its height in twips.
    #[must_use]
    pub fn font_class(mut self, name: &str, height: u16) -> Self {
        self.class = Some((name.to_owned(), height));
        self
    }

    /// Sets the text colour as RGBA.
    #[must_use]
    pub const fn color(mut self, color: [u8; 4]) -> Self {
        self.color = Some(color);
        self
    }

    /// Wraps lines at the width of the field.
    #[must_use]
    pub const fn word_wrap(mut self) -> Self {
        self.flags |= 0x40;
        self
    }

    /// Allows more than one line.
    #[must_use]
    pub const fn multiline(mut self) -> Self {
        self.flags |= 0x20;
        self
    }

    /// Shows asterisks instead of the text.
    #[must_use]
    pub const fn password(mut self) -> Self {
        self.flags |= 0x10;
        self
    }

    /// Sets alignment (0 left, 1 right, 2 centre), margins, indent and leading in twips.
    #[must_use]
    pub const fn layout(
        mut self,
        align: u8,
        margins: (u16, u16),
        indent: u16,
        leading: i16,
    ) -> Self {
        self.layout = Some((align, margins.0, margins.1, indent, leading));
        self
    }

    /// Returns the body of the tag.
    #[must_use]
    pub fn body(&self) -> Vec<u8> {
        let first = self.flags
            | u8::from(self.text.is_some()) << 7
            | u8::from(self.color.is_some()) << 2
            | u8::from(self.font.is_some());
        let second =
            u8::from(self.class.is_some()) << 7 | u8::from(self.layout.is_some()) << 5 | self.html;
        let mut bits = BitWriter::default();
        bits.bytes(&self.id.to_le_bytes());
        rect(&mut bits, self.bounds);
        bits.bytes(&[first, second]);
        if let Some((font, _)) = self.font {
            bits.bytes(&font.to_le_bytes());
        }
        if let Some((name, _)) = &self.class {
            bits.bytes(name.as_bytes());
            bits.bytes(&[0]);
        }
        if let Some(height) = self
            .font
            .map(|font| font.1)
            .or_else(|| self.class.as_ref().map(|class| class.1))
        {
            bits.bytes(&height.to_le_bytes());
        }
        if let Some(color) = self.color {
            bits.bytes(&color);
        }
        if let Some((align, left, right, indent, leading)) = self.layout {
            bits.bytes(&[align]);
            for value in [left, right, indent] {
                bits.bytes(&value.to_le_bytes());
            }
            bits.bytes(&leading.to_le_bytes());
        }
        bits.bytes(&[0]);
        if let Some(text) = &self.text {
            bits.bytes(text.as_bytes());
            bits.bytes(&[0]);
        }
        bits.finish()
    }
}
