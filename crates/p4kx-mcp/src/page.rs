// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::hash::{DefaultHasher, Hash, Hasher};

use crate::error::ToolError;

/// The page size when a call gives none.
pub(crate) const DEFAULT_LIMIT: usize = 100;
/// The largest page size; a larger `limit` is lowered to it.
pub(crate) const MAX_LIMIT: usize = 1000;
const MAX_OFFSET_LEN: usize = 20;
const LISTING_LEN: usize = 16;
const MAX_CURSOR_LEN: usize = MAX_OFFSET_LEN + 1 + LISTING_LEN;

/// One page of a listing: the items from `offset`, at most `limit` of them.
///
/// A cursor is `<offset>-<listing>`, where the listing is a hash of the tool and its arguments,
/// so a cursor only continues the listing that returned it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Page {
    offset: usize,
    limit: usize,
    listing: u64,
}

impl Page {
    /// Creates the page a call of the listing `listing` asks for with its `cursor` and `limit`.
    pub(crate) fn new(
        listing: &impl Hash,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Self, ToolError> {
        let mut hasher = DefaultHasher::new();
        listing.hash(&mut hasher);
        let listing = hasher.finish();
        let offset = match cursor {
            None => 0,
            Some(text) => {
                parse(text, listing).ok_or_else(|| ToolError::BadCursor(excerpt(text)))?
            }
        };
        let limit = limit.map_or(DEFAULT_LIMIT, |limit| {
            usize::try_from(limit)
                .unwrap_or(MAX_LIMIT)
                .clamp(1, MAX_LIMIT)
        });
        Ok(Self {
            offset,
            limit,
            listing,
        })
    }

    /// Returns whether the item at `index` of the listing is on this page.
    pub(crate) const fn contains(self, index: usize) -> bool {
        index >= self.offset && index - self.offset < self.limit
    }

    /// Returns the cursor of the next page of a listing of `total` items, or `None` on the last
    /// page; an offset beyond `total` is a bad cursor.
    pub(crate) fn next_cursor(self, total: usize) -> Result<Option<String>, ToolError> {
        self.next_cursor_after(self.limit, total)
    }

    /// Returns the cursor of the page after the first `taken` items of this page, of a listing
    /// of `total` items, or `None` when no item follows; an offset beyond `total` is a bad
    /// cursor.
    pub(crate) fn next_cursor_after(
        self,
        taken: usize,
        total: usize,
    ) -> Result<Option<String>, ToolError> {
        if self.offset > total {
            return Err(ToolError::BadCursor(self.cursor(self.offset)));
        }
        let end = self.offset.saturating_add(taken);
        Ok((end < total).then(|| self.cursor(end)))
    }

    /// Returns the position of the first item of the page in the listing.
    pub(crate) const fn offset(self) -> usize {
        self.offset
    }

    /// Returns the page size.
    pub(crate) const fn limit(self) -> usize {
        self.limit
    }

    /// Returns the items of `items` on this page.
    pub(crate) fn slice<T>(self, items: &[T]) -> &[T] {
        let start = self.offset.min(items.len());
        let end = start.saturating_add(self.limit).min(items.len());
        items.get(start..end).unwrap_or_default()
    }

    fn cursor(self, offset: usize) -> String {
        format!("{offset}-{:0width$x}", self.listing, width = LISTING_LEN)
    }
}

fn parse(text: &str, listing: u64) -> Option<usize> {
    let (offset, key) = text.split_once('-')?;
    let valid = (1..=MAX_OFFSET_LEN).contains(&offset.len())
        && offset.bytes().all(|byte| byte.is_ascii_digit())
        && key.len() == LISTING_LEN
        && u64::from_str_radix(key, 16).ok()? == listing;
    valid.then(|| offset.parse().ok()).flatten()
}

fn excerpt(text: &str) -> String {
    text.chars().take(MAX_CURSOR_LEN).collect()
}

#[cfg(test)]
mod tests {
    use super::{MAX_LIMIT, Page};

    #[test]
    fn limits_are_clamped_to_the_page_size_range() {
        assert_eq!(
            Page::new(&"a", None, Some(0)).map(Page::limit).ok(),
            Some(1),
            "at least one"
        );
        assert_eq!(
            Page::new(&"a", None, Some(u32::MAX)).map(Page::limit).ok(),
            Some(MAX_LIMIT),
            "at most the cap"
        );
    }

    #[test]
    fn a_cursor_continues_only_its_own_listing() -> Result<(), Box<dyn std::error::Error>> {
        let first = Page::new(&"a", None, Some(2))?;
        let cursor = first.next_cursor(5)?.ok_or("a next page")?;
        let second = Page::new(&"a", Some(&cursor), Some(2))?;
        assert!(second.contains(2) && !second.contains(1), "the second page");
        assert!(
            Page::new(&"b", Some(&cursor), None).is_err(),
            "another listing rejects it"
        );
        Ok(())
    }

    #[test]
    fn malformed_cursors_are_rejected() {
        for cursor in [
            "",
            "-1",
            "1e3",
            "abc",
            "2-",
            "-0000000000000000",
            "123456789012345678901-0000000000000000",
        ] {
            assert!(
                Page::new(&"a", Some(cursor), None).is_err(),
                "{cursor:?} is rejected"
            );
        }
    }
}
