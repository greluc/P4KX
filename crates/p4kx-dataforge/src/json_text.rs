use std::io::{self, Write};

const INDENT: &[u8; 64] = b"                                                                ";

/// Writes a line ending and `depth` levels of two spaces.
pub(crate) fn new_line(out: &mut impl Write, depth: usize) -> io::Result<()> {
    out.write_all(b"\n")?;
    let mut spaces = depth.saturating_mul(2);
    while spaces > 0 {
        let chunk = spaces.min(INDENT.len());
        out.write_all(INDENT.get(..chunk).unwrap_or_default())?;
        spaces -= chunk;
    }
    Ok(())
}
