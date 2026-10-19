use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

use super::table::LocalizationTable;

const CHECK_EVERY: usize = 4096;

/// A search text, folded to lower case once, matched against keys and texts without allocating.
#[derive(Debug, Clone)]
pub(super) struct Needle {
    ascii: bool,
    folded: Box<[char]>,
    bytes: Box<[u8]>,
}

impl Needle {
    /// Returns the needle of `query`, ignoring a leading `@`; `None` when nothing is left, which
    /// matches every row.
    pub(super) fn new(query: &str) -> Option<Self> {
        let query = query.strip_prefix('@').unwrap_or(query);
        if query.is_empty() {
            return None;
        }
        let folded: Box<[char]> = query.chars().flat_map(char::to_lowercase).collect();
        Some(Self {
            ascii: query.is_ascii(),
            bytes: query
                .bytes()
                .map(|byte| byte.to_ascii_lowercase())
                .collect(),
            folded,
        })
    }

    /// Returns whether `haystack` contains the needle, ignoring case.
    pub(super) fn is_in(&self, haystack: &str) -> bool {
        if self.ascii {
            self.is_in_bytes(haystack.as_bytes())
        } else {
            haystack.char_indices().any(|(at, _)| {
                let mut folded = haystack
                    .get(at..)
                    .unwrap_or_default()
                    .chars()
                    .flat_map(char::to_lowercase);
                self.folded
                    .iter()
                    .all(|&wanted| folded.next() == Some(wanted))
            })
        }
    }

    fn is_in_bytes(&self, haystack: &[u8]) -> bool {
        let Some((&first, _)) = self.bytes.split_first() else {
            return true;
        };
        let length = self.bytes.len();
        let mut from = 0;
        while let Some(rest) = haystack.get(from..)
            && rest.len() >= length
        {
            let starts = rest.get(..=rest.len() - length).unwrap_or_default();
            let Some(at) = starts
                .iter()
                .position(|byte| byte.to_ascii_lowercase() == first)
            else {
                return false;
            };
            if rest
                .get(at..at + length)
                .is_some_and(|window| window.eq_ignore_ascii_case(&self.bytes))
            {
                return true;
            }
            from += at + 1;
        }
        false
    }
}

/// Returns the rows whose key or text in any language contains `needle`, in row order; `None`
/// once `cancelled` is set.
pub(super) fn matching(
    table: &LocalizationTable,
    needle: &Needle,
    cancelled: &AtomicBool,
) -> Option<Vec<u32>> {
    let mut hit = vec![false; table.len()];
    let by_key: Vec<u32> = (0..table.len())
        .into_par_iter()
        .filter(|&row| {
            u32::try_from(row)
                .ok()
                .and_then(|row| table.key(row))
                .is_some_and(|key| needle.is_in(key))
        })
        .filter_map(|row| u32::try_from(row).ok())
        .collect();
    let by_text: Vec<Vec<u32>> = table
        .languages()
        .par_iter()
        .map(|&language| {
            table
                .texts()
                .iter(language)
                .enumerate()
                .filter_map(|(count, (key, text))| {
                    if count % CHECK_EVERY == 0 && cancelled.load(Ordering::Relaxed) {
                        return None;
                    }
                    if needle.is_in(text) {
                        table.position(key)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .collect();
    if cancelled.load(Ordering::Relaxed) {
        return None;
    }
    for row in by_key.into_iter().chain(by_text.into_iter().flatten()) {
        if let Some(slot) = usize::try_from(row).ok().and_then(|row| hit.get_mut(row)) {
            *slot = true;
        }
    }
    Some(
        hit.iter()
            .enumerate()
            .filter(|&(_, &matched)| matched)
            .filter_map(|(row, _)| u32::try_from(row).ok())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::Needle;

    fn finds(query: &str, haystack: &str) -> bool {
        Needle::new(query).is_some_and(|needle| needle.is_in(haystack))
    }

    #[test]
    fn an_ascii_needle_matches_ignoring_case() {
        assert!(finds("rifle", "Item_Name_RIFLE_01"), "inside");
        assert!(finds("RIF", "a rifle"), "folded needle");
        assert!(!finds("rifles", "a rifle"), "longer than the rest");
        assert!(!finds("x", ""), "empty haystack");
        assert!(finds("a", "xxa"), "at the end");
        assert!(finds("aab", "aaab"), "after a false start");
    }

    #[test]
    fn a_needle_with_non_ascii_characters_matches_ignoring_case() {
        assert!(finds("größe", "Die GRÖSSE ist GRÖẞE"), "umlaut folded");
        assert!(finds("ÄRZ", "Ärzte"), "start");
        assert!(!finds("ärz", "Arzt"), "different letter");
    }

    #[test]
    fn a_leading_at_sign_is_ignored_and_an_empty_needle_matches_everything() {
        assert!(finds("@item_name", "item_Name_x"), "@ dropped");
        assert!(Needle::new("").is_none(), "empty");
        assert!(Needle::new("@").is_none(), "only @");
    }
}
