use crate::error::ToolError;

/// The page size when a call gives none.
pub(crate) const DEFAULT_LIMIT: usize = 100;
/// The largest page size; a larger `limit` is lowered to it.
pub(crate) const MAX_LIMIT: usize = 1000;
const MAX_CURSOR_LEN: usize = 20;

/// One page of a listing: the items from `offset`, at most `limit` of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Page {
    offset: usize,
    limit: usize,
}

impl Page {
    /// Creates the page a call asks for with its `cursor` and `limit`.
    pub(crate) fn new(cursor: Option<&str>, limit: Option<u32>) -> Result<Self, ToolError> {
        let offset = match cursor {
            None => 0,
            Some(text) => {
                let parsed = (text.len() <= MAX_CURSOR_LEN
                    && text.bytes().all(|b| b.is_ascii_digit()))
                .then(|| text.parse::<usize>().ok())
                .flatten();
                parsed.ok_or_else(|| ToolError::BadCursor(cursor_excerpt(text)))?
            }
        };
        let limit = limit.map_or(DEFAULT_LIMIT, |limit| {
            usize::try_from(limit)
                .unwrap_or(MAX_LIMIT)
                .clamp(1, MAX_LIMIT)
        });
        Ok(Self { offset, limit })
    }

    /// Returns whether the item at `index` of the listing is on this page.
    pub(crate) const fn contains(self, index: usize) -> bool {
        index >= self.offset && index - self.offset < self.limit
    }

    /// Returns the cursor of the next page of a listing of `total` items, or `None` on the last
    /// page; an offset beyond `total` is a bad cursor.
    pub(crate) fn next_cursor(self, total: usize) -> Result<Option<String>, ToolError> {
        if self.offset > total {
            return Err(ToolError::BadCursor(self.offset.to_string()));
        }
        let end = self.offset.saturating_add(self.limit);
        Ok((end < total).then(|| end.to_string()))
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
}

fn cursor_excerpt(text: &str) -> String {
    text.chars().take(MAX_CURSOR_LEN).collect()
}

#[cfg(test)]
mod tests {
    use super::{MAX_LIMIT, Page};

    #[test]
    fn limits_are_clamped_to_the_page_size_range() {
        assert_eq!(
            Page::new(None, Some(0)).map(Page::limit).ok(),
            Some(1),
            "at least one"
        );
        assert_eq!(
            Page::new(None, Some(u32::MAX)).map(Page::limit).ok(),
            Some(MAX_LIMIT),
            "at most the cap"
        );
    }

    #[test]
    fn only_decimal_cursors_are_accepted() {
        for cursor in ["", "-1", "1e3", "abc", "123456789012345678901"] {
            assert!(
                Page::new(Some(cursor), None).is_err(),
                "{cursor:?} is rejected"
            );
        }
    }
}
