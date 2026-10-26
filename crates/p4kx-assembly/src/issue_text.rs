// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// Returns the message of the attachments of a character definition that are not drawn.
pub(crate) fn attachments_left_out(files: &[String]) -> String {
    let shown: Vec<&str> = files.iter().take(3).map(String::as_str).collect();
    let more = files.len().saturating_sub(shown.len());
    let mut text = format!(
        "the character definition's attachments are not drawn: {}",
        shown.join(", ")
    );
    if more > 0 {
        text = format!("{text} and {more} more");
    }
    text
}

/// Returns the message of `count` animations that are not written for `reason`.
pub(crate) fn animations_left_out(count: usize, reason: &str, examples: &[String]) -> String {
    let mut text = if count == 1 {
        format!("the animation is not written: {reason}")
    } else {
        format!("{count} animations are not written: {reason}")
    };
    if count > 1 && !examples.is_empty() {
        text.push_str(" (e.g. ");
        text.push_str(&examples.join(", "));
        text.push(')');
    }
    text
}
