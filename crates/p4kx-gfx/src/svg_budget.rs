use std::io::{self, Write};

/// A writer that fails once more than a limit of bytes would pass through it.
pub(crate) struct Capped<W> {
    inner: W,
    left: u64,
    exceeded: bool,
}

impl<W: Write> Capped<W> {
    /// Wraps `inner`, letting `limit` bytes through.
    pub(crate) const fn new(inner: W, limit: u64) -> Self {
        Self {
            inner,
            left: limit,
            exceeded: false,
        }
    }

    /// Returns whether a write was refused for passing the limit.
    pub(crate) const fn exceeded(&self) -> bool {
        self.exceeded
    }
}

impl<W: Write> Write for Capped<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = u64::try_from(buf.len()).unwrap_or(u64::MAX);
        if let Some(rest) = self.left.checked_sub(len) {
            self.left = rest;
            self.inner.write(buf)
        } else {
            self.exceeded = true;
            Err(io::Error::other("the SVG document exceeds its size limit"))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
