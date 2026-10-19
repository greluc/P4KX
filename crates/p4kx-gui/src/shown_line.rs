use std::fmt::Write as _;
use std::ops::Range;

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId};

use crate::theme::palette;

const MAX_LINE_CHARS: usize = 1024;
const MAX_LINE_BYTES: usize = MAX_LINE_CHARS * 4;
const TAB: &str = "    ";
const CUT: char = '…';

/// One line as it is shown: its text and the byte ranges of the text shown as escapes.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ShownLine {
    pub(crate) text: String,
    escapes: Vec<Range<usize>>,
}

impl ShownLine {
    /// Returns the line as a layout job in `font`: escapes in `color-gray-2-text`, the rest in
    /// the colour the painter passes.
    pub(crate) fn job(&self, font: &FontId) -> LayoutJob {
        escaped_job(&self.text, &self.escapes, font)
    }

    /// Returns the byte ranges of the text that are escapes.
    pub(crate) fn escapes(&self) -> &[Range<usize>] {
        &self.escapes
    }
}

/// Returns `text` as a layout job in `font` with the byte ranges `escapes` in `color-gray-2-text`
/// and the rest in the colour the painter passes.
pub(crate) fn escaped_job(text: &str, escapes: &[Range<usize>], font: &FontId) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut at = 0;
    let format = |color| TextFormat::simple(font.clone(), color);
    for escape in escapes {
        job.append(
            text.get(at..escape.start).unwrap_or_default(),
            0.0,
            format(Color32::PLACEHOLDER),
        );
        job.append(
            text.get(escape.clone()).unwrap_or_default(),
            0.0,
            format(palette::GRAY_2_TEXT),
        );
        at = escape.end;
    }
    job.append(
        text.get(at..).unwrap_or_default(),
        0.0,
        format(Color32::PLACEHOLDER),
    );
    job
}

/// Returns `bytes` as one line for display: without its terminator, tabs as spaces, control and
/// bidirectional formatting characters as escapes, and cut after [`MAX_LINE_CHARS`] characters.
///
/// Only the first [`MAX_LINE_BYTES`] bytes are decoded, so a huge line costs no more than a
/// short one.
pub(crate) fn display(bytes: &[u8]) -> ShownLine {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    let head = bytes.get(..MAX_LINE_BYTES).unwrap_or(bytes);
    let mut shown = ShownLine::default();
    let mut cut = head.len() < bytes.len();
    for (count, character) in String::from_utf8_lossy(head).chars().enumerate() {
        if count == MAX_LINE_CHARS {
            cut = true;
            break;
        }
        push(&mut shown, character);
    }
    if cut {
        shown.text.push(CUT);
    }
    shown
}

/// Returns `text` as one line for display like [`display`], with each line feed as the escape
/// `\n`, and cut after [`MAX_LINE_CHARS`] characters however long the text is.
pub(crate) fn display_text(text: &str) -> ShownLine {
    let mut shown = ShownLine::default();
    let mut characters = text.chars();
    for character in characters.by_ref().take(MAX_LINE_CHARS) {
        if character == '\n' {
            let start = shown.text.len();
            shown.text.push_str("\\n");
            shown.escapes.push(start..shown.text.len());
        } else {
            push(&mut shown, character);
        }
    }
    if characters.next().is_some() {
        shown.text.push(CUT);
    }
    shown
}

fn push(shown: &mut ShownLine, character: char) {
    if character == '\t' {
        shown.text.push_str(TAB);
    } else if is_hidden(character) {
        let start = shown.text.len();
        let code = u32::from(character);
        let _infallible = if code < 0x80 {
            write!(shown.text, "\\x{code:02X}")
        } else {
            write!(shown.text, "\\u{{{code:04X}}}")
        };
        shown.escapes.push(start..shown.text.len());
    } else {
        shown.text.push(character);
    }
}

/// Returns whether `character` is a C0 or C1 control, DEL, or a bidirectional formatting
/// character, none of which shows as itself.
const fn is_hidden(character: char) -> bool {
    matches!(
        character,
        '\u{0}'..='\u{1F}'
            | '\u{7F}'..='\u{9F}'
            | '\u{61C}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2066}'..='\u{2069}'
    )
}

#[cfg(test)]
mod tests {
    use super::{display, display_text};

    #[test]
    fn a_text_is_shown_on_one_line_with_its_line_feeds_as_escapes() {
        let shown = display_text("Rolls.\nSlowly.\u{202E}\u{1B}");

        assert_eq!(shown.text, "Rolls.\\nSlowly.\\u{202E}\\x1B", "escaped");
        assert_eq!(shown.escapes, [6..8, 15..23, 23..27], "marked as escapes");
    }

    #[test]
    fn a_huge_text_is_cut_after_a_bounded_number_of_characters() {
        let shown = display_text(&"x".repeat(10 * 1024 * 1024));

        assert_eq!(shown.text.chars().count(), 1025, "cut after 1024");
        assert!(shown.text.ends_with('…'), "ellipsis");
    }

    #[test]
    fn a_line_is_shown_without_its_terminator_and_with_tabs_as_spaces() {
        assert_eq!(display(b"\ta=1\r\n").text, "    a=1", "terminator and tab");
        assert_eq!(
            display(b"\xFFx").text,
            "\u{FFFD}x",
            "invalid UTF-8 replaced"
        );
    }

    #[test]
    fn control_and_bidi_characters_are_shown_as_escapes() {
        let shown = display("a\u{1B}b\u{202E}c\u{85}".as_bytes());

        assert_eq!(shown.text, "a\\x1Bb\\u{202E}c\\u{0085}", "escaped");
        assert_eq!(
            shown.escapes,
            [1..5, 6..14, 15..23],
            "the escapes are marked for the muted colour"
        );
    }

    #[test]
    fn a_very_long_line_is_cut_with_an_ellipsis() {
        let shown = display(&[b'x'; 5000]);

        assert_eq!(
            shown.text.chars().count(),
            1025,
            "cut after 1024 characters"
        );
        assert!(shown.text.ends_with('…'), "ellipsis");
    }

    #[test]
    fn a_huge_invalid_line_is_cut_before_it_is_decoded() {
        let line = vec![0xFF_u8; 10 * 1024 * 1024];

        let shown = display(&line);

        assert_eq!(
            shown.text.chars().count(),
            1025,
            "cut after 1024 characters"
        );
        assert!(shown.text.starts_with('\u{FFFD}'), "replaced");
        assert!(shown.text.ends_with('…'), "ellipsis");
    }

    #[test]
    fn a_line_cut_inside_its_bytes_is_marked_as_cut() {
        let line = "😀".repeat(1100);

        let shown = display(line.as_bytes());

        assert_eq!(
            shown.text.chars().count(),
            1025,
            "1024 characters fill the bytes read"
        );
        assert!(shown.text.ends_with('…'), "cut by its bytes");
    }
}
