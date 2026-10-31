// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{Text, Texts};

/// Returns the line `text` of the Record at `path` with `guid` and `reason`; the message takes
/// `path`, `guid` and `reason`.
pub(super) fn record_line(
    texts: &Texts,
    text: impl Into<Text>,
    path: &str,
    guid: &str,
    reason: &str,
) -> String {
    texts.format(
        text,
        &[
            ("path", path.into()),
            ("guid", guid.into()),
            ("reason", reason.into()),
        ],
    )
}
