// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Text made safe to print on one line.

/// Returns `text` on one line: line breaks and tabs escaped, other control characters replaced.
#[must_use]
pub fn one_line(text: &str) -> String {
    let mut line = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\n' => line.push_str("\\n"),
            '\r' => line.push_str("\\r"),
            '\t' => line.push_str("\\t"),
            c if c.is_control() => line.push('\u{fffd}'),
            c => line.push(c),
        }
    }
    line
}
