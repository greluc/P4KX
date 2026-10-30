// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::collections::hash_map::Entry as Slot;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

/// The files of one run by their path folded to lower case, so two Entries whose paths differ only
/// in case are found whatever file system the target is on.
#[derive(Debug, Default)]
pub(crate) struct CaseClaims {
    claimed: Mutex<HashMap<String, usize>>,
}

impl CaseClaims {
    /// Claims `relative` for the Entry at `index`; returns the index of the Entry that holds the
    /// same folded path when that is another one.
    pub(crate) fn claim(&self, relative: &Path, index: usize) -> Option<usize> {
        let key = relative.to_string_lossy().to_lowercase();
        let mut claimed = self.claimed.lock().unwrap_or_else(PoisonError::into_inner);
        match claimed.entry(key) {
            Slot::Occupied(holder) if *holder.get() != index => Some(*holder.get()),
            Slot::Occupied(_) => None,
            Slot::Vacant(free) => {
                free.insert(index);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CaseClaims;
    use std::path::Path;

    #[test]
    fn the_first_entry_keeps_a_path_that_differs_only_in_case() {
        let claims = CaseClaims::default();
        assert_eq!(claims.claim(Path::new("Data/A.txt"), 4), None);
        assert_eq!(claims.claim(Path::new("data/a.TXT"), 9), Some(4));
        assert_eq!(claims.claim(Path::new("Data/A.txt"), 4), None, "same Entry");
        assert_eq!(claims.claim(Path::new("Data/b.txt"), 9), None);
    }
}
