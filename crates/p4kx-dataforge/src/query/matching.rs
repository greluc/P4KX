/// Returns whether `needle` occurs in `text`, ignoring ASCII case.
pub(crate) fn contains(text: &str, needle: &str) -> bool {
    needle.is_empty()
        || text
            .as_bytes()
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

/// Returns whether `text` starts with `prefix`, ignoring ASCII case.
pub(crate) fn starts_with(text: &str, prefix: &str) -> bool {
    text.as_bytes()
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix.as_bytes()))
}

/// Returns whether all of `text` matches the glob `pattern`, ignoring ASCII case: `*` matches
/// any run of characters and `?` exactly one. Matching allocates nothing.
pub(crate) fn like(text: &str, pattern: &str) -> bool {
    let (mut token, mut position) = (0_usize, 0_usize);
    let mut resume: Option<(usize, usize)> = None;
    loop {
        let wanted = pattern.get(token..).and_then(|rest| rest.chars().next());
        let found = text.get(position..).and_then(|rest| rest.chars().next());
        match (wanted, found) {
            (None, None) => return true,
            (Some('*'), _) => {
                token = token.saturating_add(1);
                resume = Some((token, position));
                continue;
            }
            (Some('?'), Some(c)) => {
                token = token.saturating_add(1);
                position = position.saturating_add(c.len_utf8());
                continue;
            }
            (Some(wanted), Some(c)) if wanted.eq_ignore_ascii_case(&c) => {
                token = token.saturating_add(wanted.len_utf8());
                position = position.saturating_add(c.len_utf8());
                continue;
            }
            _ => {}
        }
        let Some((after_star, start)) = resume else {
            return false;
        };
        let Some(skipped) = text.get(start..).and_then(|rest| rest.chars().next()) else {
            return false;
        };
        let start = start.saturating_add(skipped.len_utf8());
        resume = Some((after_star, start));
        token = after_star;
        position = start;
    }
}
