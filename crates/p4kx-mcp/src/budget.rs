use std::io::{self, Write};

use serde::Serialize;

use crate::error::ToolError;
use crate::page::Page;

/// The most bytes of JSON the items of one DataForge result may take.
pub(crate) const MAX_RESPONSE_LEN: usize = 256 << 10;

/// What is left of the response cap while a page is filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Budget {
    left: usize,
}

impl Budget {
    /// Creates the budget of one response, [`MAX_RESPONSE_LEN`] bytes.
    pub(crate) const fn new() -> Self {
        Self {
            left: MAX_RESPONSE_LEN,
        }
    }

    /// Returns the bytes left.
    pub(crate) const fn left(self) -> usize {
        self.left
    }

    /// Takes `len` bytes; returns `false` and takes nothing when they do not fit.
    pub(crate) const fn take(&mut self, len: usize) -> bool {
        if len > self.left {
            return false;
        }
        self.left -= len;
        true
    }
}

/// Fills one page from `items`, the items of a listing of `total` from the page's offset on:
/// at most the page size, and no more than fit [`MAX_RESPONSE_LEN`] as JSON together. Returns
/// them with the cursor of the next page; `describe` names an item that does not fit alone.
pub(crate) fn fill<T: Serialize>(
    page: Page,
    total: usize,
    items: impl IntoIterator<Item = Result<T, ToolError>>,
    describe: impl Fn(&T) -> String,
) -> Result<(Vec<T>, Option<String>), ToolError> {
    let mut budget = Budget::new();
    let mut taken = Vec::new();
    for item in items.into_iter().take(page.limit()) {
        let item = item?;
        if !budget.take(json_len(&item)) {
            if taken.is_empty() {
                return Err(ToolError::TooLargeItem {
                    what: describe(&item),
                    limit: MAX_RESPONSE_LEN,
                    hint: "ask for it by a narrower filter",
                });
            }
            break;
        }
        taken.push(item);
    }
    let next = page.next_cursor_after(taken.len(), total)?;
    Ok((taken, next))
}

/// Returns the length of `item` as JSON, without writing it anywhere.
pub(crate) fn json_len(item: &impl Serialize) -> usize {
    let mut counter = Counter::default();
    serde_json::to_writer(&mut counter, item).map_or(usize::MAX, |()| counter.len)
}

/// A sink that counts the bytes written to it.
#[derive(Debug, Default)]
pub(crate) struct Counter {
    len: usize,
}

impl Counter {
    /// Returns the bytes written so far.
    pub(crate) const fn len(&self) -> usize {
        self.len
    }
}

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.len = self.len.saturating_add(bytes.len());
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A buffer that refuses to grow beyond its limit, so a large value stops being written as
/// soon as it cannot fit.
#[derive(Debug)]
pub(crate) struct Capped {
    bytes: Vec<u8>,
    limit: usize,
}

impl Capped {
    /// Creates an empty buffer of at most `limit` bytes.
    pub(crate) const fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }

    /// Returns the bytes written.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for Capped {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "the response cap is reached",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::{Budget, Capped, MAX_RESPONSE_LEN, json_len};

    #[test]
    fn the_budget_takes_what_fits_and_nothing_else() {
        let mut budget = Budget::new();
        assert!(budget.take(MAX_RESPONSE_LEN - 10), "most of it");
        assert!(!budget.take(11), "one byte too many");
        assert_eq!(budget.left(), 10, "nothing taken");
        assert!(budget.take(10), "the rest");
    }

    #[test]
    fn json_lengths_count_the_written_bytes() {
        assert_eq!(json_len(&"a\"b"), 6, "quotes and an escape");
        assert_eq!(json_len(&[1, 22]), 6, "an array");
    }

    #[test]
    fn a_capped_buffer_refuses_to_pass_its_limit() {
        let mut capped = Capped::new(4);
        assert!(capped.write_all(b"abc").is_ok(), "fits");
        assert!(capped.write_all(b"de").is_err(), "does not fit");
        assert_eq!(capped.into_bytes(), b"abc", "nothing of the refused write");
    }
}
