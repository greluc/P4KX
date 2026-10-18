use std::ffi::OsString;
use std::path::{Path, PathBuf};

use p4kx_texture::{DdsHeader, RgbaImage, cube_cross, write_png};
use rayon::prelude::*;

use crate::output_file::write_replacing;
use crate::{ConversionError, Texture};

const CUBE_FACES: u32 = 6;

/// The names of a cubemap's faces in DDS order +X, −X, +Y, −Y, +Z, −Z, as the face files of
/// [`CubemapLayout::Faces`] carry them.
pub const CUBE_FACE_NAMES: [&str; 6] = ["posx", "negx", "posy", "negy", "posz", "negz"];

/// How a PNG export writes a cubemap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CubemapLayout {
    /// One file per face, `<stem>_posx.<extension>` … `<stem>_negz.<extension>` next to the
    /// requested file.
    #[default]
    Faces,
    /// One horizontal cross of all six faces in the requested file.
    Cross,
}

/// What a PNG export wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PngReport {
    /// The exported mip level, 0 being the largest.
    pub level: u32,
    /// Width of each PNG in pixels.
    pub width: u32,
    /// Height of each PNG in pixels.
    pub height: u32,
    /// The written files: one, or six faces of a cubemap.
    pub files: Vec<PathBuf>,
    /// Whether an alpha tail was merged into the alpha channel.
    pub alpha_tail: bool,
}

impl Texture<'_> {
    /// Decodes mip `level`, 0 being the largest, and writes it to the file `out` as an 8-bit RGBA
    /// PNG, replacing it; a cubemap is written as six face files.
    ///
    /// Everything is decoded before anything is written, and each file is written under a
    /// temporary name first, so a failed file leaves no partial file behind; when writing one face
    /// fails, faces already written stay.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Texture::png_to_file_as`].
    pub fn png_to_file(&self, out: &Path, level: u32) -> Result<PngReport, ConversionError> {
        self.png_to_file_as(out, level, CubemapLayout::Faces)
    }

    /// Decodes mip `level`, 0 being the largest, with its alpha tail, and writes it as 8-bit RGBA
    /// PNG to the file `out`, or a cubemap as `layout` says; existing files are replaced.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::OutputIsInput`] when a file to write is the header or one of
    /// the parts, [`ConversionError::Write`] when a file cannot be written, a
    /// [`ConversionError`] wrapping [`p4kx_texture::TextureError::UnsupportedImage`] for a
    /// volume, a texture array or a cubemap array, and the errors of
    /// [`Texture::decode_surfaces`].
    pub fn png_to_file_as(
        &self,
        out: &Path,
        level: u32,
        layout: CubemapLayout,
    ) -> Result<PngReport, ConversionError> {
        let cube = is_cube(self.header());
        let files: Vec<PathBuf> = if cube && layout == CubemapLayout::Faces {
            CUBE_FACE_NAMES
                .iter()
                .map(|face| face_path(out, face))
                .collect()
        } else {
            vec![out.to_path_buf()]
        };
        for file in &files {
            self.refuse_input(file)?;
        }
        let images = match (cube, layout) {
            (false, _) => vec![self.decode_mip(level)?],
            (true, CubemapLayout::Faces) => self.decode_surfaces(level)?,
            (true, CubemapLayout::Cross) => vec![
                cube_cross(&self.decode_surfaces(level)?)
                    .map_err(|error| ConversionError::of_texture(self.name(), error))?,
            ],
        };
        files
            .par_iter()
            .zip(&images)
            .try_for_each(|(file, image)| self.write_png_file(file, image))?;
        let (width, height) = images
            .first()
            .map_or((0, 0), |image| (image.width(), image.height()));
        Ok(PngReport {
            level,
            width,
            height,
            files,
            alpha_tail: self.has_alpha_tail(),
        })
    }

    fn write_png_file(&self, out: &Path, image: &RgbaImage) -> Result<(), ConversionError> {
        write_replacing(out, |writer| {
            write_png(image, writer)
                .map_err(|error| ConversionError::of_texture(self.name(), error))
        })
    }
}

/// Returns whether header describes one cubemap, not an array of them.
pub(crate) const fn is_cube(header: &DdsHeader) -> bool {
    header.faces() == CUBE_FACES && header.array_size() == 1
}

/// Returns `out` with `_<face>` appended to its file stem.
pub(crate) fn face_path(out: &Path, face: &str) -> PathBuf {
    let mut name = OsString::from(out.file_stem().unwrap_or_default());
    name.push("_");
    name.push(face);
    if let Some(extension) = out.extension() {
        name.push(".");
        name.push(extension);
    }
    out.with_file_name(name)
}
