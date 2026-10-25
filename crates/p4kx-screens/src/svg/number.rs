// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

pub(super) const MAX_MAGNITUDE: f32 = 1.0e7;

/// Reads numbers, flags and command letters from the text of a path, a transform or a point list.
pub(super) struct Scanner<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Scanner<'a> {
    pub(super) const fn new(text: &'a str) -> Self {
        Self { text, at: 0 }
    }

    fn rest(&self) -> &'a str {
        self.text.get(self.at..).unwrap_or_default()
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    pub(super) fn skip_space(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.at += 1;
        }
    }

    pub(super) fn skip_separator(&mut self) {
        self.skip_space();
        if self.peek() == Some(',') {
            self.at += 1;
            self.skip_space();
        }
    }

    pub(super) fn at_end(&mut self) -> bool {
        self.skip_space();
        self.peek().is_none()
    }

    pub(super) fn eat(&mut self, wanted: char) -> bool {
        self.skip_space();
        if self.peek() == Some(wanted) {
            self.at += wanted.len_utf8();
            true
        } else {
            false
        }
    }

    pub(super) fn letter(&mut self) -> Option<char> {
        self.skip_space();
        let c = self.peek().filter(char::is_ascii_alphabetic)?;
        self.at += c.len_utf8();
        Some(c)
    }

    pub(super) fn starts_number(&mut self) -> bool {
        self.skip_space();
        self.peek()
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.'))
    }

    pub(super) fn flag(&mut self) -> Option<bool> {
        self.skip_space();
        let flag = match self.peek()? {
            '0' => false,
            '1' => true,
            _ => return None,
        };
        self.at += 1;
        self.skip_separator();
        Some(flag)
    }

    pub(super) fn number(&mut self) -> Option<f32> {
        self.skip_space();
        let rest = self.rest();
        let bytes = rest.as_bytes();
        let mut end = 0;
        if matches!(bytes.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        let digits = |from: usize| {
            bytes
                .get(from..)
                .unwrap_or_default()
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count()
        };
        let whole = digits(end);
        end += whole;
        let mut fraction = 0;
        if bytes.get(end) == Some(&b'.') {
            fraction = digits(end + 1);
            end += 1 + fraction;
        }
        if whole == 0 && fraction == 0 {
            return None;
        }
        if matches!(bytes.get(end), Some(b'e' | b'E')) {
            let mut at = end + 1;
            if matches!(bytes.get(at), Some(b'+' | b'-')) {
                at += 1;
            }
            let exponent = digits(at);
            if exponent > 0 {
                end = at + exponent;
            }
        }
        let value: f32 = rest.get(..end)?.parse().ok()?;
        self.at += end;
        self.skip_separator();
        (value.is_finite() && value.abs() <= MAX_MAGNITUDE).then_some(value)
    }
}

/// Parses the leading number of `text`, which may be followed by a unit.
pub(super) fn leading_number(text: &str) -> Option<f32> {
    let mut scanner = Scanner::new(text.trim());
    scanner.number()
}

/// Parses a list of numbers separated by spaces or commas; `None` when anything else is in it.
pub(super) fn number_list(text: &str, limit: usize) -> Option<Vec<f32>> {
    let mut scanner = Scanner::new(text);
    let mut values = Vec::new();
    while !scanner.at_end() {
        if values.len() >= limit {
            return None;
        }
        values.push(scanner.number()?);
    }
    Some(values)
}
