use std::borrow::Cow;

/// Returns `text` made safe to print on a terminal: control characters, line and paragraph
/// separators and the characters that reorder text are replaced by U+FFFD. Names in the Archive
/// are untrusted.
pub(crate) fn printable(text: &str) -> Cow<'_, str> {
    if !text.chars().any(is_unsafe) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(
        text.chars()
            .map(|c| if is_unsafe(c) { '\u{fffd}' } else { c })
            .collect(),
    )
}

const fn is_unsafe(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{200e}'
                | '\u{200f}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}

#[cfg(test)]
mod tests {
    use super::printable;

    #[test]
    fn controls_and_reordering_characters_are_replaced() {
        assert_eq!(printable("plain/Name_01.dds"), "plain/Name_01.dds");
        assert_eq!(printable("a\u{1b}[2Jb"), "a\u{fffd}[2Jb");
        assert_eq!(printable("a\nb\u{0}c"), "a\u{fffd}b\u{fffd}c");
        assert_eq!(printable("x\u{202e}gnp.exe"), "x\u{fffd}gnp.exe");
        assert_eq!(printable("ä ö"), "ä ö");
    }
}
