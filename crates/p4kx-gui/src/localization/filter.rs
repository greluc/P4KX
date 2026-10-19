use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

use super::table::LocalizationTable;

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

/// Returns the rows whose key or the shown text in any language contains `needle`, in row
/// order; `None` once `cancelled` is set. Both passes stop scanning as soon as it is set.
pub(super) fn matching(
    table: &LocalizationTable,
    needle: &Needle,
    cancelled: &AtomicBool,
) -> Option<Vec<u32>> {
    let by_key = rows_by_key(table, needle, cancelled);
    let by_text = rows_by_text(table, needle, cancelled);
    if cancelled.load(Ordering::Relaxed) {
        return None;
    }
    let mut hit = vec![false; table.len()];
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

fn rows_by_key(table: &LocalizationTable, needle: &Needle, cancelled: &AtomicBool) -> Vec<u32> {
    (0..table.len())
        .into_par_iter()
        .take_any_while(|_| !cancelled.load(Ordering::Relaxed))
        .filter_map(|row| u32::try_from(row).ok())
        .filter(|&row| table.key(row).is_some_and(|key| needle.is_in(key)))
        .collect()
}

fn rows_by_text(
    table: &LocalizationTable,
    needle: &Needle,
    cancelled: &AtomicBool,
) -> Vec<Vec<u32>> {
    table
        .languages()
        .par_iter()
        .map(|&language| {
            table
                .texts()
                .iter(language)
                .take_while(|_| !cancelled.load(Ordering::Relaxed))
                .filter(|&(key, text)| {
                    needle.is_in(text) && table.texts().is_translated(key, language)
                })
                .filter_map(|(key, _)| table.position(key))
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use p4kx_extraction::Language;

    use super::{Needle, matching, rows_by_key, rows_by_text};
    use crate::localization::table::LocalizationTable;

    const ENGLISH: &str = "item_a=alpha\nitem_b=\nitem_c=beta\nitem_d=alphabet\n";
    const GERMAN: &str = "item_a=alpha\nitem_b=hidden alpha\nLOC_EMPTY=zzz\n";

    fn table() -> LocalizationTable {
        LocalizationTable::from_inis(&[(Language::English, ENGLISH), (Language::German, GERMAN)])
    }

    fn needle(query: &str) -> Needle {
        Needle::new(query).unwrap_or_else(|| unreachable!("a non-empty query"))
    }

    #[test]
    fn a_cancelled_search_returns_none_and_both_passes_scan_nothing() {
        let table = table();
        let cancelled = AtomicBool::new(true);
        let needle = needle("item");
        assert_eq!(matching(&table, &needle, &cancelled), None, "result");
        assert!(rows_by_key(&table, &needle, &cancelled).is_empty(), "keys");
        assert!(
            rows_by_text(&table, &needle, &cancelled)
                .iter()
                .all(Vec::is_empty),
            "texts"
        );
        let running = AtomicBool::new(false);
        assert_eq!(matching(&table, &needle, &running), Some(vec![0, 1, 2, 3]));
    }

    #[test]
    fn a_text_the_cell_shows_as_missing_is_not_matched() {
        let table = table();
        let running = AtomicBool::new(false);
        assert_eq!(
            matching(&table, &needle("hidden"), &running),
            Some(vec![1]),
            "a translated German text is found"
        );
        assert_eq!(
            matching(&table, &needle("zzz"), &running),
            Some(vec![]),
            "a placeholder text is not"
        );
    }

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
