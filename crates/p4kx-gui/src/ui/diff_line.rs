const MAX_LINE_CHARS: usize = 4096;
const TAB: &str = "    ";

/// Returns `bytes` as one line for display: without its terminator, tabs as spaces and cut after
/// [`MAX_LINE_CHARS`] characters.
pub(super) fn display(bytes: &[u8]) -> String {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    let text = String::from_utf8_lossy(bytes);
    let mut shown = String::with_capacity(text.len().min(MAX_LINE_CHARS));
    for (count, character) in text.chars().enumerate() {
        if count == MAX_LINE_CHARS {
            shown.push('…');
            break;
        }
        if character == '\t' {
            shown.push_str(TAB);
        } else {
            shown.push(character);
        }
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::display;

    #[test]
    fn a_line_is_shown_without_its_terminator_and_with_tabs_as_spaces() {
        assert_eq!(display(b"\ta=1\r\n"), "    a=1", "terminator and tab");
        assert_eq!(display(b"\xFFx"), "\u{FFFD}x", "invalid UTF-8 replaced");
    }

    #[test]
    fn a_very_long_line_is_cut_with_an_ellipsis() {
        let shown = display(&[b'x'; 5000]);

        assert_eq!(shown.chars().count(), 4097, "cut after 4096 characters");
        assert!(shown.ends_with('…'), "ellipsis");
    }
}
