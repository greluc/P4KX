use std::hash::{Hash, Hasher};

const CHUNK: usize = 64;

/// A localisation key that hashes and compares ignoring ASCII case.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Key<'a>(pub(crate) &'a str);

impl PartialEq for Key<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(other.0)
    }
}

impl Eq for Key<'_> {}

impl Hash for Key<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut buffer = [0_u8; CHUNK];
        for chunk in self.0.as_bytes().chunks(CHUNK) {
            if let Some(folded) = buffer.get_mut(..chunk.len()) {
                for (slot, byte) in folded.iter_mut().zip(chunk) {
                    *slot = byte.to_ascii_lowercase();
                }
                state.write(folded);
            }
        }
        state.write_u8(0xff);
    }
}
