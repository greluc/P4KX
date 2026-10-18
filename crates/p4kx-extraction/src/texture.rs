use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use p4kx_archive::Archive;
use p4kx_texture::{
    DdsHeader, MAX_HEADER_LEN, MergeReport, Mip, RgbaImage, TextureError, merge, read_mip,
    write_png,
};

use crate::output_file::{read_error, write_replacing};
use crate::texture_parts::{PartReader, Parts, is_same_file};
use crate::{ConversionError, UnknownEntries};

/// What a PNG export wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PngReport {
    /// The exported mip level, 0 being the largest.
    pub level: u32,
    /// Width of the PNG in pixels.
    pub width: u32,
    /// Height of the PNG in pixels.
    pub height: u32,
}

/// A DDS texture whose header is an Entry of an Archive or a file on disk, with its sibling parts
/// `.1` … `.N` next to it.
#[derive(Debug)]
pub struct Texture<'a> {
    name: String,
    header_size: u64,
    header: DdsHeader,
    parts: Parts<'a>,
}

impl<'a> Texture<'a> {
    /// Opens the texture whose header is the Entry at `path`, matched with either separator and
    /// ignoring ASCII case.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::UnknownEntry`] when no Entry has that path, and another
    /// [`ConversionError`] when the Entry cannot be read or holds no DDS header.
    pub fn from_archive(archive: &'a Archive, path: &str) -> Result<Self, ConversionError> {
        let parts = Parts::in_archive(archive, path)
            .ok_or_else(|| UnknownEntries(vec![path.to_owned()]))?;
        let Parts::Archive { header, .. } = &parts else {
            return Err(UnknownEntries(vec![path.to_owned()]).into());
        };
        let name = header.path().to_owned();
        let header_size = header.uncompressed_size();
        let reader = archive.open_entry(header)?;
        let header = parse_start(reader, &name)?;
        Ok(Self {
            name,
            header_size,
            header,
            parts,
        })
    }

    /// Returns the name of the header: its Entry path or its path on disk.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the parsed DDS header.
    #[must_use]
    pub const fn header(&self) -> &DdsHeader {
        &self.header
    }

    /// Returns the number of sibling parts the header needs.
    ///
    /// # Errors
    ///
    /// Returns a [`TextureError`] when the format's size is unknown or the header's tail is not a
    /// run of whole mips.
    pub fn part_count(&self) -> Result<u32, TextureError> {
        self.header.part_count(self.header_size)
    }

    /// Writes the merged texture as one standard DDS file to `out`.
    ///
    /// # Errors
    ///
    /// Returns a [`ConversionError`] naming the part when a part is missing, truncated, oversized
    /// or unreadable, and another one when the texture cannot be merged or `out` fails.
    pub fn merge_into(&self, out: &mut impl Write) -> Result<MergeReport, ConversionError> {
        let header = self.open_header()?;
        merge(header, self.header_size, &self.parts, out)
            .map_err(|error| ConversionError::of_texture(&self.name, error))
    }

    /// Writes the merged texture to the file `out`, replacing it; the file is written under a
    /// temporary name first, so a failed merge leaves nothing behind.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::OutputIsInput`] when `out` is the header or one of its parts,
    /// [`ConversionError::Write`] when the file cannot be written, and the errors of
    /// [`Texture::merge_into`].
    pub fn merge_to_file(&self, out: &Path) -> Result<MergeReport, ConversionError> {
        self.refuse_input(out)?;
        write_replacing(out, |writer| self.merge_into(writer))
    }

    /// Reads mip `level`, 0 being the largest, from its part or from the header.
    ///
    /// # Errors
    ///
    /// Returns a [`ConversionError`] naming the part when the mip's part is missing, truncated,
    /// oversized or unreadable, and another one when the level does not exist or the texture has
    /// several images per mip.
    pub fn read_mip(&self, level: u32) -> Result<Mip, ConversionError> {
        let header = self.open_header()?;
        read_mip(header, self.header_size, &self.parts, level)
            .map_err(|error| ConversionError::of_texture(&self.name, error))
    }

    /// Reads and decodes mip `level`, 0 being the largest, to 8-bit RGBA.
    ///
    /// # Errors
    ///
    /// Returns a [`ConversionError`] wrapping [`TextureError::UnsupportedDecode`] before reading
    /// anything when the format has no decoder, and the errors of [`Texture::read_mip`].
    pub fn decode_mip(&self, level: u32) -> Result<RgbaImage, ConversionError> {
        let format = self.header.format();
        if !format.is_decodable() {
            return Err(ConversionError::of_texture(
                &self.name,
                TextureError::UnsupportedDecode(format),
            ));
        }
        self.read_mip(level)?
            .decode()
            .map_err(|error| ConversionError::of_texture(&self.name, error))
    }

    /// Decodes mip `level`, 0 being the largest, and writes it to the file `out` as an 8-bit RGBA
    /// PNG, replacing it; a failed export leaves nothing behind.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::OutputIsInput`] when `out` is the header or one of its parts,
    /// [`ConversionError::Write`] when the file cannot be written, and the errors of
    /// [`Texture::decode_mip`].
    pub fn png_to_file(&self, out: &Path, level: u32) -> Result<PngReport, ConversionError> {
        self.refuse_input(out)?;
        let image = self.decode_mip(level)?;
        write_replacing(out, |writer| {
            write_png(&image, writer)
                .map_err(|error| ConversionError::of_texture(&self.name, error))
        })?;
        Ok(PngReport {
            level,
            width: image.width(),
            height: image.height(),
        })
    }

    fn refuse_input(&self, out: &Path) -> Result<(), ConversionError> {
        if is_same_file(out, self.input_files()) {
            return Err(ConversionError::OutputIsInput {
                path: out.to_path_buf(),
            });
        }
        Ok(())
    }

    fn open_header(&self) -> Result<PartReader<'_>, ConversionError> {
        match &self.parts {
            Parts::Archive {
                archive, header, ..
            } => Ok(PartReader::Entry(archive.open_entry(header)?)),
            Parts::Files { header } => File::open(header)
                .map(PartReader::File)
                .map_err(|source| read_error(header, source)),
        }
    }

    fn input_files(&self) -> Vec<PathBuf> {
        match &self.parts {
            Parts::Archive { .. } => Vec::new(),
            Parts::Files { header } => {
                let count = self.part_count().unwrap_or(0);
                std::iter::once(header.clone())
                    .chain((1..=count).filter_map(|index| self.parts.file_of(index)))
                    .collect()
            }
        }
    }
}

impl Texture<'static> {
    /// Opens the texture whose header is the file at `path`, with its parts `<path>.1` … next to
    /// it.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::NoSuchFile`] when the file does not exist, and another
    /// [`ConversionError`] when it cannot be read or holds no DDS header.
    pub fn from_file(path: &Path) -> Result<Self, ConversionError> {
        let file = File::open(path).map_err(|source| {
            if source.kind() == io::ErrorKind::NotFound {
                ConversionError::NoSuchFile {
                    path: path.to_path_buf(),
                }
            } else {
                read_error(path, source)
            }
        })?;
        let header_size = file
            .metadata()
            .map_err(|source| read_error(path, source))?
            .len();
        let name = path.display().to_string();
        let header = parse_start(file, &name)?;
        Ok(Self {
            name,
            header_size,
            header,
            parts: Parts::Files {
                header: path.to_path_buf(),
            },
        })
    }
}

fn parse_start(reader: impl Read, name: &str) -> Result<DdsHeader, ConversionError> {
    let mut start = Vec::with_capacity(MAX_HEADER_LEN);
    reader
        .take(MAX_HEADER_LEN as u64)
        .read_to_end(&mut start)
        .map_err(|source| ConversionError::of_texture(name, TextureError::ReadHeader(source)))?;
    DdsHeader::parse(&start).map_err(|error| ConversionError::of_texture(name, error))
}
