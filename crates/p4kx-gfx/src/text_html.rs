use crate::geometry::Rgba;

/// The most characters of one text field that are laid out and drawn; the rest is dropped.
pub const MAX_TEXT_CHARS: usize = 4096;

const MAX_STYLES: usize = 256;
const MAX_DEPTH: usize = 32;
const MAX_TAG_LEN: usize = 512;
const MAX_ENTITY_LEN: usize = 12;
const TWIPS_PER_PIXEL: f32 = 20.0;
const MAX_PIXELS: f32 = 1000.0;

/// What the markup around a character states about it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Style {
    pub(crate) face: Option<String>,
    pub(crate) size: Option<f32>,
    pub(crate) color: Option<Rgba>,
    pub(crate) letter_spacing: f32,
    pub(crate) kerning: bool,
    pub(crate) align: Option<u8>,
}

/// One character of a text with the index of its style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cell {
    pub(crate) ch: char,
    pub(crate) style: u16,
}

/// A text as characters with styles; newlines are `'\n'`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Parsed {
    pub(crate) cells: Vec<Cell>,
    pub(crate) styles: Vec<Style>,
    pub(crate) truncated: bool,
}

struct Builder {
    cells: Vec<Cell>,
    styles: Vec<Style>,
    truncated: bool,
    after_return: bool,
}

impl Builder {
    fn new() -> Self {
        Self {
            cells: Vec::new(),
            styles: vec![Style::default()],
            truncated: false,
            after_return: false,
        }
    }

    const fn full(&self) -> bool {
        self.cells.len() >= MAX_TEXT_CHARS
    }

    fn push(&mut self, ch: char, style: u16) {
        if self.full() {
            self.truncated = true;
            return;
        }
        let after_return = std::mem::take(&mut self.after_return);
        let ch = match ch {
            '\r' => {
                self.after_return = true;
                '\n'
            }
            '\n' if after_return => return,
            '\t' => ' ',
            ch if ch.is_control() && ch != '\n' => return,
            ch => ch,
        };
        self.cells.push(Cell { ch, style });
    }

    fn push_str(&mut self, text: &str, style: u16) {
        for ch in text.chars() {
            if self.full() {
                self.truncated = true;
                return;
            }
            self.push(ch, style);
        }
    }

    fn intern(&mut self, style: &Style) -> u16 {
        let at = self
            .styles
            .iter()
            .position(|known| known == style)
            .unwrap_or_else(|| {
                if self.styles.len() < MAX_STYLES {
                    self.styles.push(style.clone());
                    self.styles.len() - 1
                } else {
                    MAX_STYLES - 1
                }
            });
        u16::try_from(at).unwrap_or(0)
    }

    fn finish(self) -> Parsed {
        Parsed {
            cells: self.cells,
            styles: self.styles,
            truncated: self.truncated,
        }
    }
}

/// Parses plain text.
pub(crate) fn plain(text: &str) -> Parsed {
    let mut builder = Builder::new();
    builder.push_str(text, 0);
    builder.finish()
}

/// Parses the Flash subset of HTML: `p`, `br` and `font`; other tags only group, entities are
/// decoded.
pub(crate) fn html(text: &str) -> Parsed {
    let mut builder = Builder::new();
    let mut stack = vec![Style::default()];
    let mut overflow = 0_usize;
    let mut current = 0_u16;
    let mut rest = text;
    while !rest.is_empty() && !builder.full() {
        let Some(at) = rest.find(['<', '&']) else {
            builder.push_str(rest, current);
            break;
        };
        builder.push_str(rest.get(..at).unwrap_or_default(), current);
        rest = rest.get(at..).unwrap_or_default();
        if rest.starts_with('<') {
            let end = rest
                .bytes()
                .take(MAX_TAG_LEN + 1)
                .position(|byte| byte == b'>');
            let Some(end) = end else {
                builder.push('<', current);
                rest = rest.get(1..).unwrap_or_default();
                continue;
            };
            let body = rest.get(1..end).unwrap_or_default();
            apply_tag(body, &mut stack, &mut overflow, &mut builder, current);
            current = stack.last().map_or(0, |style| builder.intern(style));
            rest = rest.get(end + 1..).unwrap_or_default();
        } else {
            let entity = rest
                .bytes()
                .skip(1)
                .take(MAX_ENTITY_LEN + 1)
                .position(|byte| byte == b';')
                .and_then(|end| Some((end, entity(rest.get(1..=end)?)?)));
            if let Some((end, ch)) = entity {
                builder.push(ch, current);
                rest = rest.get(end + 2..).unwrap_or_default();
            } else {
                builder.push('&', current);
                rest = rest.get(1..).unwrap_or_default();
            }
        }
    }
    while builder.cells.last().is_some_and(|cell| cell.ch == '\n') {
        builder.cells.pop();
    }
    builder.finish()
}

fn apply_tag(
    body: &str,
    stack: &mut Vec<Style>,
    overflow: &mut usize,
    builder: &mut Builder,
    current: u16,
) {
    let closing = body.starts_with('/');
    let self_closing = body.ends_with('/');
    let body = body.trim_start_matches('/').trim_end_matches('/');
    let name_end = body
        .find(|ch: char| ch.is_ascii_whitespace())
        .unwrap_or(body.len());
    let name = body.get(..name_end).unwrap_or_default();
    let attributes = body.get(name_end..).unwrap_or_default();
    if closing {
        if name.eq_ignore_ascii_case("p") {
            builder.push('\n', current);
        }
        if *overflow > 0 {
            *overflow -= 1;
        } else if stack.len() > 1 {
            stack.pop();
        }
        return;
    }
    if name.eq_ignore_ascii_case("br") {
        builder.push('\n', current);
        return;
    }
    if self_closing || name.is_empty() {
        return;
    }
    if stack.len() >= MAX_DEPTH {
        *overflow += 1;
        return;
    }
    let mut style = stack.last().cloned().unwrap_or_default();
    if name.eq_ignore_ascii_case("p") {
        for (key, value) in attribute_pairs(attributes) {
            if key.eq_ignore_ascii_case("align") {
                style.align = align(value).or(style.align);
            }
        }
    } else if name.eq_ignore_ascii_case("font") {
        font_attributes(&mut style, attributes);
    }
    stack.push(style);
}

fn font_attributes(style: &mut Style, attributes: &str) {
    for (key, value) in attribute_pairs(attributes) {
        if key.eq_ignore_ascii_case("face") {
            style.face = Some(value.chars().take(MAX_TAG_LEN).collect());
        } else if key.eq_ignore_ascii_case("size") {
            if let Some(pixels) = number(value).filter(|_| !value.starts_with(['+', '-'])) {
                style.size = Some(pixels.clamp(1.0, MAX_PIXELS) * TWIPS_PER_PIXEL);
            }
        } else if key.eq_ignore_ascii_case("color") {
            style.color = color(value).or(style.color);
        } else if key.eq_ignore_ascii_case("letterspacing") {
            style.letter_spacing = number(value)
                .map_or(0.0, |pixels| pixels.clamp(-MAX_PIXELS, MAX_PIXELS))
                * TWIPS_PER_PIXEL;
        } else if key.eq_ignore_ascii_case("kerning") {
            style.kerning = value == "1" || value.eq_ignore_ascii_case("true");
        }
    }
}

fn number(value: &str) -> Option<f32> {
    value.trim().parse::<f32>().ok().filter(|v| v.is_finite())
}

fn align(value: &str) -> Option<u8> {
    ["left", "right", "center", "justify"]
        .iter()
        .position(|name| value.eq_ignore_ascii_case(name))
        .and_then(|at| u8::try_from(at).ok())
}

fn color(value: &str) -> Option<Rgba> {
    let digits = value.strip_prefix('#')?;
    let packed = u32::from_str_radix(digits, 16)
        .ok()
        .filter(|_| digits.len() == 6)?;
    let [_, red, green, blue] = packed.to_be_bytes();
    Some(Rgba {
        red,
        green,
        blue,
        alpha: u8::MAX,
    })
}

fn attribute_pairs(text: &str) -> Vec<(&str, &str)> {
    let mut pairs = Vec::new();
    let mut rest = text.trim_start();
    while let Some(equals) = rest.find('=') {
        let key = rest.get(..equals).unwrap_or_default().trim();
        let after = rest.get(equals + 1..).unwrap_or_default().trim_start();
        let (value, tail) = split_value(after);
        let key = key.rsplit(|ch: char| ch.is_ascii_whitespace()).next();
        pairs.extend(key.map(|key| (key, value)));
        rest = tail.trim_start();
        if pairs.len() >= 16 {
            break;
        }
    }
    pairs
}

fn split_value(after: &str) -> (&str, &str) {
    if let Some(quote @ ('"' | '\'')) = after.chars().next() {
        let inner = after.get(1..).unwrap_or_default();
        return inner.find(quote).map_or((inner, ""), |end| {
            (
                inner.get(..end).unwrap_or_default(),
                inner.get(end + 1..).unwrap_or_default(),
            )
        });
    }
    let end = after
        .find(|ch: char| ch.is_ascii_whitespace())
        .unwrap_or(after.len());
    (
        after.get(..end).unwrap_or_default(),
        after.get(end..).unwrap_or_default(),
    )
}

fn entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        _ => {
            let digits = name.strip_prefix('#')?;
            let code = match digits.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => digits.parse().ok()?,
            };
            char::from_u32(code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_TEXT_CHARS, html, plain};

    fn text(parsed: &super::Parsed) -> String {
        parsed.cells.iter().map(|cell| cell.ch).collect()
    }

    #[test]
    fn plain_text_normalises_line_breaks_and_drops_controls() {
        assert_eq!(text(&plain("a\r\nb\rc\nd\u{1}e\tf")), "a\nb\nc\nde f");
    }

    #[test]
    fn html_applies_font_and_paragraph_attributes() {
        let parsed = html(
            "<p align=\"right\"><font face='Orbitron' size=\"12\" color=\"#FF8000\" letterSpacing=\"1\" kerning=\"1\">A&amp;B&#67;&#x44;</font></p>",
        );
        assert_eq!(text(&parsed), "A&BCD");
        let style = &parsed.styles[usize::from(parsed.cells[0].style)];
        assert_eq!(style.face.as_deref(), Some("Orbitron"));
        assert_eq!(style.size, Some(240.0));
        assert_eq!(style.color.map(|color| color.green), Some(128));
        assert_eq!(style.letter_spacing, 20.0);
        assert!(style.kerning);
        assert_eq!(style.align, Some(1));
    }

    #[test]
    fn html_breaks_lines_and_ignores_unknown_markup() {
        let parsed = html("<b>a</b><br/>b<p>c</p><p>d</p><x y='<'>e");
        assert_eq!(text(&parsed), "a\nbc\nd\ne");
        assert_eq!(text(&html("1 < 2 & 3 &bogus;")), "1 < 2 & 3 &bogus;");
        assert_eq!(text(&html("<font")), "<font");
    }

    #[test]
    fn the_text_and_the_markup_are_bounded() {
        let long = "x".repeat(MAX_TEXT_CHARS + 10);
        let parsed = plain(&long);
        assert_eq!(parsed.cells.len(), MAX_TEXT_CHARS);
        assert!(parsed.truncated);
        let nested = "<font size='9'>".repeat(5000) + "t";
        let parsed = html(&nested);
        assert_eq!(text(&parsed), "t");
        assert!(parsed.styles.len() <= 256);
    }
}
