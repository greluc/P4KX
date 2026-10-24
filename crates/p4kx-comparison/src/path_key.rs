// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cmp::Ordering;

const fn fold(byte: u8) -> u8 {
    if byte == b'/' {
        b'\\'
    } else {
        byte.to_ascii_lowercase()
    }
}

/// Orders Entry paths by their normalised form: ASCII case folded, `/` read as `\`.
pub(crate) fn cmp_normalised(a: &str, b: &str) -> Ordering {
    a.bytes().map(fold).cmp(b.bytes().map(fold))
}

/// Returns whether two Entry paths are equal in their normalised form.
pub(crate) fn eq_normalised(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().map(fold).eq(b.bytes().map(fold))
}

/// Returns the top-level directory of an Entry path, or `""` for an Entry at the root.
pub(crate) fn top_level(path: &str) -> &str {
    path.split_once(['\\', '/'])
        .map_or("", |(directory, _)| directory)
}
