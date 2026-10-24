// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// Builds a `global.ini` localisation file as the game stores it: a UTF-8 byte order mark, then
/// one `key=value` line per text, each ending in CRLF.
#[derive(Debug, Clone, Default)]
pub struct GlobalIniBuilder {
    lines: Vec<(String, String)>,
}

impl GlobalIniBuilder {
    /// Creates a builder without texts.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the line `key=value`; `key` may carry a suffix such as `,P`.
    #[must_use]
    pub fn text(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.lines.push((key.into(), value.into()));
        self
    }

    /// Returns the bytes of the file.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        let mut bytes = b"\xef\xbb\xbf".to_vec();
        for (key, value) in &self.lines {
            bytes.extend_from_slice(key.as_bytes());
            bytes.push(b'=');
            bytes.extend_from_slice(value.as_bytes());
            bytes.extend_from_slice(b"\r\n");
        }
        bytes
    }
}
