use std::error::Error;
use std::fmt;
use std::io::{self, Write};

use crate::CancelToken;

const HEADER_LEN: usize = 12;
const LENGTH_AT: usize = 8;

#[derive(Debug)]
struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("cancelled")
    }
}

impl Error for Cancelled {}

/// Passes a GLB stream on, reporting the bytes written against the length the header states and
/// failing the next write once `cancel` is set.
pub(crate) struct Observed<'a, W> {
    inner: W,
    cancel: &'a CancelToken,
    progress: &'a dyn Fn(u64, u64),
    header: [u8; HEADER_LEN],
    seen: usize,
    written: u64,
}

impl<'a, W: Write> Observed<'a, W> {
    pub(crate) const fn new(
        inner: W,
        cancel: &'a CancelToken,
        progress: &'a dyn Fn(u64, u64),
    ) -> Self {
        Self {
            inner,
            cancel,
            progress,
            header: [0; HEADER_LEN],
            seen: 0,
            written: 0,
        }
    }

    fn total(&self) -> u64 {
        self.header
            .get(LENGTH_AT..HEADER_LEN)
            .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            .map_or(0, |bytes| u64::from(u32::from_le_bytes(bytes)))
    }

    fn remember(&mut self, bytes: &[u8]) {
        let room = HEADER_LEN.saturating_sub(self.seen);
        let take = room.min(bytes.len());
        if let (Some(target), Some(source)) = (
            self.header.get_mut(self.seen..self.seen + take),
            bytes.get(..take),
        ) {
            target.copy_from_slice(source);
        }
        self.seen += take;
    }
}

impl<W: Write> Write for Observed<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.cancel.is_cancelled() {
            return Err(io::Error::other(Cancelled));
        }
        let count = self.inner.write(buf)?;
        if let Some(written) = buf.get(..count) {
            self.remember(written);
        }
        self.written += u64::try_from(count).unwrap_or(u64::MAX);
        (self.progress)(self.written, self.total().max(self.written));
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Returns whether `error` is the failure [`Observed`] raises on cancellation.
pub(crate) fn is_cancelled(error: &io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(|inner| inner.downcast_ref::<Cancelled>().is_some())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::io::Write;

    use super::{Observed, is_cancelled};
    use crate::CancelToken;

    #[test]
    fn progress_follows_the_length_in_the_header() -> Result<(), std::io::Error> {
        let seen = Cell::new((0, 0));
        let token = CancelToken::new();
        let progress = |done, total| seen.set((done, total));
        let mut out = Observed::new(Vec::new(), &token, &progress);
        let mut bytes = b"glTF".to_vec();
        bytes.extend_from_slice(&2_u32.to_le_bytes());
        bytes.extend_from_slice(&40_u32.to_le_bytes());
        out.write_all(&bytes)?;
        assert_eq!(seen.get(), (12, 40), "header read");
        out.write_all(&[0; 8])?;
        assert_eq!(seen.get(), (20, 40), "bytes counted");
        Ok(())
    }

    #[test]
    fn a_cancelled_token_fails_the_next_write_with_the_marker() {
        let token = CancelToken::new();
        let progress = |_, _| {};
        let mut out = Observed::new(Vec::new(), &token, &progress);
        token.cancel();

        let error = out.write_all(b"x").err();

        assert!(error.is_some_and(|error| is_cancelled(&error)), "marker");
    }
}
