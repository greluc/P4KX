// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::min_part_len;

#[test]
fn no_more_parts_than_threads_are_made() {
    for records in [0, 1, 2, 31, 32, 33, 1_000, 117_709] {
        for threads in [0, 1, 2, 3, 8, 32, 64] {
            let len = min_part_len(records, threads);
            assert!(len >= 1, "{records} Records, {threads} threads: {len}");
            assert!(
                records.div_ceil(len) <= threads.max(1),
                "{records} Records, {threads} threads: parts of at least {len}"
            );
        }
    }
}
