use super::bits::{BitReader, BitWriter, copy, ilog};
use super::codebook::{self, Codebooks, Library, Shape};
use super::{HEADER_SETUP, VorbisError};

const FLOOR_TYPE_1: u32 = 1;
const MAX_RESIDUE_TYPE: u32 = 2;

/// What decoding the audio packets needs from a rebuilt setup header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Setup {
    /// The block flag of each mode: `true` for a long block.
    pub(crate) long_modes: Vec<bool>,
}

/// Rebuilds the Vorbis setup header from the Wwise setup packet `packet` into `out`, taking the
/// codebooks from `codebooks`, and checks every reference the decoder will follow.
pub(crate) fn rebuild(
    packet: &[u8],
    channels: u32,
    codebooks: Codebooks,
    out: &mut Vec<u8>,
) -> Result<Setup, VorbisError> {
    out.clear();
    out.push(HEADER_SETUP);
    out.extend_from_slice(b"vorbis");
    let mut input = BitReader::new(packet);
    let mut writer = BitWriter::new(out);
    let shapes = read_codebooks(&mut input, &mut writer, codebooks)?;
    writer.write(0, 6);
    writer.write(0, 16);
    let floors = read_floors(&mut input, &mut writer, &shapes)?;
    let residues = read_residues(&mut input, &mut writer, &shapes)?;
    let mappings = read_mappings(&mut input, &mut writer, channels, floors, residues)?;
    let long_modes = read_modes(&mut input, &mut writer, mappings)?;
    writer.write(1, 1);
    writer.finish();
    let used = input.bits_read().div_ceil(8);
    if used != packet.len() {
        return Err(VorbisError::SetupLength {
            used,
            len: packet.len(),
        });
    }
    Ok(Setup { long_modes })
}

fn field(
    input: &mut BitReader<'_>,
    out: &mut BitWriter<'_>,
    bits: u32,
) -> Result<u32, VorbisError> {
    copy(input, out, bits).ok_or(VorbisError::SetupTruncated)
}

fn read_codebooks(
    input: &mut BitReader<'_>,
    out: &mut BitWriter<'_>,
    codebooks: Codebooks,
) -> Result<Vec<Shape>, VorbisError> {
    let count = field(input, out, 8)? + 1;
    let library = Library::of(codebooks);
    let mut shapes = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let shape = match &library {
            Some(library) => {
                let id = input.read(10).ok_or(VorbisError::SetupTruncated)?;
                library.rebuild(id, out)?
            }
            None => codebook::rebuild(input, out)?,
        };
        shapes.push(shape);
    }
    Ok(shapes)
}

fn book(index: u32, shapes: &[Shape], what: &'static str) -> Result<Shape, VorbisError> {
    usize::try_from(index)
        .ok()
        .and_then(|index| shapes.get(index))
        .copied()
        .ok_or(VorbisError::InvalidSetup(what))
}

fn read_floors(
    input: &mut BitReader<'_>,
    out: &mut BitWriter<'_>,
    shapes: &[Shape],
) -> Result<u32, VorbisError> {
    let count = field(input, out, 6)? + 1;
    let mut classes = Vec::new();
    let mut dimensions = Vec::new();
    let mut posts = Vec::new();
    for _ in 0..count {
        out.write(FLOOR_TYPE_1, 16);
        let partitions = field(input, out, 5)?;
        classes.clear();
        for _ in 0..partitions {
            classes.push(field(input, out, 4)?);
        }
        let class_count = classes.iter().max().map_or(0, |&max| max + 1);
        dimensions.clear();
        for _ in 0..class_count {
            dimensions.push(field(input, out, 3)? + 1);
            let subclasses = field(input, out, 2)?;
            if subclasses != 0 {
                book(
                    field(input, out, 8)?,
                    shapes,
                    "a floor masterbook does not exist",
                )?;
            }
            for _ in 0..1_u32 << subclasses {
                if let Some(index) = field(input, out, 8)?.checked_sub(1) {
                    book(index, shapes, "a floor subclass book does not exist")?;
                }
            }
        }
        field(input, out, 2)?;
        let range_bits = field(input, out, 4)?;
        posts.clear();
        posts.extend([0, 1 << range_bits]);
        for &class in &classes {
            let class_dimensions = dimensions.get(class as usize).copied().unwrap_or(0);
            for _ in 0..class_dimensions {
                posts.push(field(input, out, range_bits)?);
            }
        }
        posts.sort_unstable();
        if posts.windows(2).any(|pair| pair.first() == pair.last()) {
            return Err(VorbisError::InvalidSetup("a floor repeats an x position"));
        }
    }
    Ok(count)
}

fn read_residues(
    input: &mut BitReader<'_>,
    out: &mut BitWriter<'_>,
    shapes: &[Shape],
) -> Result<u32, VorbisError> {
    let count = field(input, out, 6)? + 1;
    let mut cascades = Vec::new();
    for _ in 0..count {
        let residue_type = input.read(2).ok_or(VorbisError::SetupTruncated)?;
        if residue_type > MAX_RESIDUE_TYPE {
            return Err(VorbisError::InvalidSetup("a residue type is above 2"));
        }
        out.write(residue_type, 16);
        let begin = field(input, out, 24)?;
        let end = field(input, out, 24)?;
        if begin > end {
            return Err(VorbisError::InvalidSetup("a residue ends before it begins"));
        }
        field(input, out, 24)?;
        let classifications = field(input, out, 6)? + 1;
        let classbook = book(
            field(input, out, 8)?,
            shapes,
            "a residue classbook does not exist",
        )?;
        let partitions = u64::from(classifications).checked_pow(classbook.dimensions);
        if partitions.is_none_or(|partitions| partitions > u64::from(classbook.entries)) {
            return Err(VorbisError::InvalidSetup(
                "a residue classbook has too few entries",
            ));
        }
        cascades.clear();
        for _ in 0..classifications {
            let low = field(input, out, 3)?;
            let high = if field(input, out, 1)? == 1 {
                field(input, out, 5)?
            } else {
                0
            };
            cascades.push(high << 3 | low);
        }
        for &cascade in &cascades {
            for bit in 0..8 {
                if cascade & (1 << bit) != 0 {
                    let book = book(
                        field(input, out, 8)?,
                        shapes,
                        "a residue book does not exist",
                    )?;
                    if !book.has_lookup {
                        return Err(VorbisError::InvalidSetup(
                            "a residue book has no value lookup",
                        ));
                    }
                }
            }
        }
    }
    Ok(count)
}

fn read_mappings(
    input: &mut BitReader<'_>,
    out: &mut BitWriter<'_>,
    channels: u32,
    floors: u32,
    residues: u32,
) -> Result<u32, VorbisError> {
    let count = field(input, out, 6)? + 1;
    let channel_bits = ilog(channels.saturating_sub(1));
    for _ in 0..count {
        out.write(0, 16);
        let submaps = if field(input, out, 1)? == 1 {
            field(input, out, 4)? + 1
        } else {
            1
        };
        if field(input, out, 1)? == 1 {
            let steps = field(input, out, 8)? + 1;
            for _ in 0..steps {
                let magnitude = field(input, out, channel_bits)?;
                let angle = field(input, out, channel_bits)?;
                if magnitude == angle || magnitude >= channels || angle >= channels {
                    return Err(VorbisError::InvalidSetup("invalid channel coupling"));
                }
            }
        }
        if field(input, out, 2)? != 0 {
            return Err(VorbisError::InvalidSetup("a reserved mapping field is set"));
        }
        if submaps > 1 {
            for _ in 0..channels {
                if field(input, out, 4)? >= submaps {
                    return Err(VorbisError::InvalidSetup(
                        "a channel names a missing submap",
                    ));
                }
            }
        }
        for _ in 0..submaps {
            field(input, out, 8)?;
            if field(input, out, 8)? >= floors {
                return Err(VorbisError::InvalidSetup("a submap names a missing floor"));
            }
            if field(input, out, 8)? >= residues {
                return Err(VorbisError::InvalidSetup(
                    "a submap names a missing residue",
                ));
            }
        }
    }
    Ok(count)
}

fn read_modes(
    input: &mut BitReader<'_>,
    out: &mut BitWriter<'_>,
    mappings: u32,
) -> Result<Vec<bool>, VorbisError> {
    let count = field(input, out, 6)? + 1;
    let mut long_modes = Vec::with_capacity(count as usize);
    for _ in 0..count {
        long_modes.push(field(input, out, 1)? == 1);
        out.write(0, 16);
        out.write(0, 16);
        if field(input, out, 8)? >= mappings {
            return Err(VorbisError::InvalidSetup("a mode names a missing mapping"));
        }
    }
    Ok(long_modes)
}
