use std::ffi::OsString;
use std::io::Write;

use p4kx_texture::{RgbaImage, TextureError, read_mip, write_png};

use crate::buffers::Buffers;
use crate::texture_convert::{Source, Step};
use crate::texture_outputs::step_of;
use crate::texture_siblings::Replay;
use crate::write::{Chunk, Destination, EntryWriter, Outcome, create_temporary, write_error};
use crate::{CUBE_FACE_NAMES, ConflictPolicy, ExtractionError, Texture};

const CUBE_FACES: u32 = 6;

impl EntryWriter<'_> {
    /// Decodes mip `level` of `source` with its alpha tail, its part taken from `buffers.data`
    /// when the merge kept it there, and writes it as PNG next to `header`: `<name>.png`, or a
    /// cubemap's six faces as `<name>_posx.png` … `<name>_negz.png`, each under the Conflict
    /// policy.
    pub(crate) fn write_pngs(
        &self,
        source: &Source<'_>,
        header: &Destination,
        level: u32,
        buffers: &mut Buffers,
    ) -> Result<Step, ExtractionError> {
        let dds = source.texture.header();
        let is_cube = dds.faces() == CUBE_FACES && dds.array_size() == 1;
        let names: Vec<OsString> = if is_cube {
            CUBE_FACE_NAMES
                .iter()
                .map(|face| png_name(header, Some(face)))
                .collect()
        } else {
            vec![png_name(header, None)]
        };
        let destinations = names
            .iter()
            .map(|name| Destination::new(header.folder.clone(), name))
            .collect::<Result<Vec<_>, _>>()?;
        let skip = self.policy == ConflictPolicy::Skip;
        if skip && destinations.iter().all(|file| file.existing.is_some()) {
            return Ok(Step::Kept);
        }
        if self.tracker.should_stop() {
            return Ok(Step::Cancelled);
        }
        let images = Self::decode_level(source, level, &buffers.data)?;
        if images.len() != destinations.len() {
            return Err(source.texture_error(TextureError::UnsupportedImage {
                images: dds.surfaces(),
            }));
        }
        let mut steps = Vec::with_capacity(images.len());
        for (image, destination) in images.iter().zip(&destinations) {
            let step = if skip && destination.existing.is_some() {
                Step::Kept
            } else {
                self.write_png_file(source, image, destination, buffers)?
            };
            if step == Step::Cancelled {
                return Ok(step);
            }
            steps.push(step);
        }
        Ok(if steps.contains(&Step::Written) {
            Step::Written
        } else {
            Step::Kept
        })
    }

    fn write_png_file(
        &self,
        source: &Source<'_>,
        image: &RgbaImage,
        destination: &Destination,
        buffers: &mut Buffers,
    ) -> Result<Step, ExtractionError> {
        let png = &mut buffers.output;
        png.clear();
        write_png(image, &mut *png).map_err(|error| source.texture_error(error))?;
        let expected = || (png.len() as u64, crc32c::crc32c(png));
        if let Some(outcome) = self.decide(destination, expected, &mut buffers.chunk)? {
            return Ok(step_of(&outcome));
        }
        if self.tracker.should_stop() {
            return Ok(Step::Cancelled);
        }
        let (temporary, mut file) = create_temporary(&destination.folder)?;
        let copied = file
            .write_all(png)
            .map(|()| Chunk::End)
            .map_err(|error| write_error(destination.file.clone(), error));
        drop(file);
        let placed =
            self.put_in_place(&temporary, &destination.file, copied, Outcome::Converted)?;
        Ok(step_of(&placed))
    }

    /// Decodes every image of mip `level` with the alpha tail merged in, replaying the part in
    /// `captured` when it holds exactly that mip.
    fn decode_level(
        source: &Source<'_>,
        level: u32,
        captured: &[u8],
    ) -> Result<Vec<RgbaImage>, ExtractionError> {
        let texture = &source.texture;
        let alpha = texture
            .checked_alpha_tail(level)
            .map_err(ExtractionError::Texture)?;
        let replayed = (level < source.part_count
            && !captured.is_empty()
            && texture.header().mip_size(level) == Some(captured.len() as u64))
        .then(|| (source.part_count - level, captured));
        let siblings = Replay {
            parts: texture.parts(),
            captured: replayed,
        };
        let header = texture.open_header().map_err(ExtractionError::Texture)?;
        let mip = read_mip(header, texture.header_size(), &siblings, level)
            .map_err(|error| source.texture_error(error))?;
        let images = texture
            .decode_mip_data(&mip)
            .map_err(ExtractionError::Texture)?;
        Texture::with_alpha_tail(alpha, level, images).map_err(ExtractionError::Texture)
    }
}

/// Returns the name of a PNG next to the header: its name with `.png` for `.dds`, and
/// `_<face>` before it for a cubemap face.
fn png_name(header: &Destination, face: Option<&str>) -> OsString {
    let mut name = header
        .file
        .file_stem()
        .map(OsString::from)
        .unwrap_or_default();
    if let Some(face) = face {
        name.push("_");
        name.push(face);
    }
    name.push(".png");
    name
}
