// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_chunks::{ByteOrder, ChunkFile, ChunkType, Container};
use p4kx_foundation::{ByteReader, ParseError};
use thiserror::Error;

use crate::Transform;
use crate::reader::to_usize;

const INCLUDED_OBJECTS: ChunkType = ChunkType::new(Container::CrCh, 0x0010);
const NAME_LEN: usize = 256;
const OBJECT_LEN: usize = 208;
const MODEL_INDEX_AT: usize = 60;
const TRANSFORM_AT: usize = 64;
const PALETTE_AT: usize = 172;
const MODEL_OBJECT: u32 = 1;
const HEADER_WORDS_LEN: usize = 28;

/// The most model paths one `.soc` file lists.
pub const MAX_INCLUDED_MODELS: usize = 1 << 16;

/// The most placed objects one `.soc` file lists.
pub const MAX_INCLUDED_OBJECTS: usize = 1 << 20;

/// A failure to read the included objects of a `.soc` file.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum IncludedObjectsError {
    /// The file is no chunk file this crate reads.
    #[error("the file is not a readable chunk file")]
    Chunks(#[from] p4kx_chunks::ChunkError),
    /// The chunk ends before a value it declares.
    #[error("the included objects chunk is truncated")]
    Truncated(#[from] ParseError),
    /// The chunk is flagged big-endian, which is not read.
    #[error("the included objects chunk is big-endian")]
    BigEndian,
    /// The chunk lists more of something than is read.
    #[error("the included objects chunk lists more than {limit} {what}")]
    TooMany {
        /// What there is too much of.
        what: &'static str,
        /// The most read.
        limit: usize,
    },
    /// The object table does not fit the chunk.
    #[error("the object table of {declared} bytes does not fit the {available} bytes left")]
    ObjectTable {
        /// The length the chunk declares.
        declared: u64,
        /// The bytes left in the chunk.
        available: u64,
    },
}

/// One model placed by a `.soc` file.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct IncludedObject {
    /// The position of the model's path in [`IncludedObjects::models`].
    pub model: usize,
    /// The transform from the model's space to the container's, in `CryEngine` coordinates, Z up
    /// and in metres.
    pub transform: Transform,
    /// The position of the tint palette in [`IncludedObjects::palettes`], if the object has one.
    pub palette: Option<usize>,
}

/// The models a `.soc` file places: the `IncludedObjects` chunk (`CrCh` type `0x0010`) with the
/// model paths and tint palettes it lists and one [`IncludedObject`] per placement.
///
/// Objects of a kind that is not read end the list; objects that name no listed model or have a
/// transform that is not finite are left out and counted.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct IncludedObjects {
    /// The model paths, as the file spells them: below `Data`, usually `.cgf` files.
    pub models: Vec<String>,
    /// The Record paths of the tint palettes.
    pub palettes: Vec<String>,
    /// The placed models in file order.
    pub objects: Vec<IncludedObject>,
    /// The number of objects left out because they name no listed model or have no usable
    /// transform.
    pub invalid: usize,
    /// The kind of the first object that is not a placed model, which ends the list.
    pub unsupported: Option<u32>,
}

impl IncludedObjects {
    /// Reads the `IncludedObjects` chunks of the `CrCh` chunk file `bytes`, a `.soc` file, and
    /// joins them; returns `None` when it holds none.
    ///
    /// # Errors
    ///
    /// Returns [`IncludedObjectsError`] when the file is no chunk file or a chunk is truncated or
    /// declares more than is read.
    pub fn read(bytes: &[u8]) -> Result<Option<Self>, IncludedObjectsError> {
        let file = ChunkFile::parse(bytes)?;
        let mut found: Option<Self> = None;
        for (index, chunk) in file.chunks_of(INCLUDED_OBJECTS) {
            if chunk.byte_order() == ByteOrder::Big {
                return Err(IncludedObjectsError::BigEndian);
            }
            let data = file.chunk_data(index).unwrap_or_default();
            let chunk = read_chunk(data)?;
            found = Some(match found {
                Some(first) => first.joined(chunk),
                None => chunk,
            });
        }
        Ok(found)
    }

    fn joined(mut self, other: Self) -> Self {
        let models = self.models.len();
        let palettes = self.palettes.len();
        self.models.extend(other.models);
        self.palettes.extend(other.palettes);
        self.objects
            .extend(other.objects.into_iter().map(|object| IncludedObject {
                model: object.model + models,
                palette: object.palette.map(|palette| palette + palettes),
                ..object
            }));
        self.invalid += other.invalid;
        self.unsupported = self.unsupported.or(other.unsupported);
        self
    }
}

fn read_chunk(data: &[u8]) -> Result<IncludedObjects, IncludedObjectsError> {
    let mut reader = ByteReader::new(data);
    reader.skip(4)?;
    let model_count = to_usize(reader.read_u32()?);
    check(model_count, MAX_INCLUDED_MODELS, "models")?;
    let models = names(&mut reader, model_count)?;
    let material_count = usize::from(reader.read_u16()?);
    let palette_count = usize::from(reader.read_u16()?);
    reader.skip(material_count.saturating_mul(NAME_LEN))?;
    let palettes = names(&mut reader, palette_count)?;
    reader.skip(HEADER_WORDS_LEN)?;
    let declared = reader.read_u32()?;
    let available = reader.remaining() as u64;
    if u64::from(declared) > available {
        return Err(IncludedObjectsError::ObjectTable {
            declared: u64::from(declared),
            available,
        });
    }
    let table = reader.read_bytes(to_usize(declared))?;
    check(table.len() / OBJECT_LEN, MAX_INCLUDED_OBJECTS, "objects")?;
    let mut objects = Vec::new();
    let mut invalid = 0;
    let mut unsupported = None;
    let (records, rest) = table.as_chunks::<OBJECT_LEN>();
    invalid += usize::from(!rest.is_empty());
    for record in records {
        let kind = ByteReader::new(record).read_u32()?;
        if kind != MODEL_OBJECT {
            unsupported = Some(kind);
            break;
        }
        match read_object(record, models.len(), palettes.len()) {
            Some(object) => objects.push(object),
            None => invalid += 1,
        }
    }
    Ok(IncludedObjects {
        models,
        palettes,
        objects,
        invalid,
        unsupported,
    })
}

const fn check(count: usize, limit: usize, what: &'static str) -> Result<(), IncludedObjectsError> {
    if count > limit {
        Err(IncludedObjectsError::TooMany { what, limit })
    } else {
        Ok(())
    }
}

fn names(reader: &mut ByteReader<'_>, count: usize) -> Result<Vec<String>, ParseError> {
    let table = reader.read_bytes(count.saturating_mul(NAME_LEN))?;
    Ok(table
        .as_chunks::<NAME_LEN>()
        .0
        .iter()
        .map(|field| {
            let name = field.split(|&byte| byte == 0).next().unwrap_or(field);
            String::from_utf8_lossy(name).trim().to_owned()
        })
        .collect())
}

fn read_object(record: &[u8], models: usize, palettes: usize) -> Option<IncludedObject> {
    let index: [u8; 2] = record
        .get(MODEL_INDEX_AT..MODEL_INDEX_AT + 2)?
        .try_into()
        .ok()?;
    let model = usize::from(u16::from_le_bytes(index));
    if model >= models {
        return None;
    }
    let mut reader = ByteReader::new(record.get(TRANSFORM_AT..)?);
    let mut rows = [[0.0_f64; 4]; 3];
    for value in rows.iter_mut().flatten() {
        *value = f64::from_bits(reader.read_u64().ok()?);
    }
    let transform = Transform::from_rows_f64(rows)?;
    let palette = record
        .get(PALETTE_AT)
        .map(|&index| usize::from(index))
        .filter(|&index| index < palettes);
    Some(IncludedObject {
        model,
        transform,
        palette,
    })
}
