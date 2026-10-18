use super::VorbisError;
use super::bits::{BitReader, BitWriter, copy, ilog};

static DEFAULT_LIBRARY: &[u8] = include_bytes!("../../codebooks/packed_codebooks.bin");
static AOTUV_LIBRARY: &[u8] = include_bytes!("../../codebooks/packed_codebooks_aoTuV_603.bin");

const SYNC: u32 = 0x56_4342;
const LENGTH_FIELD_BITS: u32 = 5;
const MAX_LENGTH_SIZE_BITS: u32 = 5;

/// The codebooks a rebuilt setup header takes, in the order they are tried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Codebooks {
    /// The codebook library of the Vorbis reference encoder that Wwise ships by default.
    Default,
    /// The codebook library of the aoTuV 6.03 encoder.
    AoTuV,
    /// Codebooks stored in the setup packet itself.
    Inline,
}

impl Codebooks {
    /// Returns a stable lower-case identifier for machine-readable output, e.g. `aotuv`.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::AoTuV => "aotuv",
            Self::Inline => "inline",
        }
    }
}

/// What the setup needs to know of a rebuilt codebook to check how it is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Shape {
    pub(crate) dimensions: u32,
    pub(crate) entries: u32,
    pub(crate) has_lookup: bool,
}

/// A codebook library: the codebooks back to back, then a table of their offsets.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Library {
    books: &'static [u8],
    offsets: &'static [u8],
}

impl Library {
    pub(crate) fn of(codebooks: Codebooks) -> Option<Self> {
        let bytes = match codebooks {
            Codebooks::Default => DEFAULT_LIBRARY,
            Codebooks::AoTuV => AOTUV_LIBRARY,
            Codebooks::Inline => return None,
        };
        let table = usize::try_from(u32::from_le_bytes(*bytes.last_chunk::<4>()?)).ok()?;
        Some(Self {
            books: bytes.get(..table)?,
            offsets: bytes.get(table..)?,
        })
    }

    fn book(&self, id: u32) -> Option<&'static [u8]> {
        let id = usize::try_from(id).ok()?;
        let offset = |index: usize| -> Option<usize> {
            let field = self
                .offsets
                .get(index.checked_mul(4)?..)?
                .first_chunk::<4>()?;
            usize::try_from(u32::from_le_bytes(*field)).ok()
        };
        self.books.get(offset(id)?..offset(id.checked_add(1)?)?)
    }

    /// Writes codebook `id` of the library as a Vorbis codebook.
    pub(crate) fn rebuild(&self, id: u32, out: &mut BitWriter<'_>) -> Result<Shape, VorbisError> {
        let book = self.book(id).ok_or(VorbisError::UnknownCodebook(id))?;
        let mut input = BitReader::new(book);
        let shape = rebuild(&mut input, out)?;
        if input.bits_read() / 8 + 1 == book.len() {
            Ok(shape)
        } else {
            Err(VorbisError::InvalidCodebook(
                "the library codebook has a different size",
            ))
        }
    }
}

/// Expands one codebook in the packed Wwise form from `input` into a Vorbis codebook.
pub(crate) fn rebuild(
    input: &mut BitReader<'_>,
    out: &mut BitWriter<'_>,
) -> Result<Shape, VorbisError> {
    let ended = || VorbisError::InvalidCodebook("the codebook ends early");
    let dimensions = input.read(4).ok_or_else(ended)?;
    let entries = input.read(14).ok_or_else(ended)?;
    if dimensions == 0 {
        return Err(VorbisError::InvalidCodebook("a codebook has no dimensions"));
    }
    out.write(SYNC, 24);
    out.write(dimensions, 16);
    out.write(entries, 24);
    let ordered = copy(input, out, 1).ok_or_else(ended)?;
    if ordered == 1 {
        copy(input, out, LENGTH_FIELD_BITS).ok_or_else(ended)?;
        let mut entry = 0_u32;
        while entry < entries {
            let count = copy(input, out, ilog(entries - entry)).ok_or_else(ended)?;
            entry = entry.saturating_add(count);
        }
        if entry > entries {
            return Err(VorbisError::InvalidCodebook("too many ordered entries"));
        }
    } else {
        let length_bits = input.read(3).ok_or_else(ended)?;
        if length_bits == 0 || length_bits > MAX_LENGTH_SIZE_BITS {
            return Err(VorbisError::InvalidCodebook("invalid codeword length size"));
        }
        let sparse = copy(input, out, 1).ok_or_else(ended)?;
        for _ in 0..entries {
            let present = if sparse == 1 {
                copy(input, out, 1).ok_or_else(ended)?
            } else {
                1
            };
            if present == 1 {
                let length = input.read(length_bits).ok_or_else(ended)?;
                out.write(length, LENGTH_FIELD_BITS);
            }
        }
    }
    let lookup = input.read(1).ok_or_else(ended)?;
    out.write(lookup, 4);
    if lookup == 1 {
        copy(input, out, 32).ok_or_else(ended)?;
        copy(input, out, 32).ok_or_else(ended)?;
        let value_bits = copy(input, out, 4).ok_or_else(ended)? + 1;
        copy(input, out, 1).ok_or_else(ended)?;
        for _ in 0..lookup1_values(entries, dimensions) {
            copy(input, out, value_bits).ok_or_else(ended)?;
        }
    }
    Ok(Shape {
        dimensions,
        entries,
        has_lookup: lookup == 1,
    })
}

/// Returns the largest number whose `dimensions`-th power does not exceed `entries`.
fn lookup1_values(entries: u32, dimensions: u32) -> u32 {
    let fits = |value: u32| {
        u64::from(value)
            .checked_pow(dimensions)
            .is_some_and(|power| power <= u64::from(entries))
    };
    let (mut low, mut high) = (0, entries);
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        if fits(middle) {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    low
}
