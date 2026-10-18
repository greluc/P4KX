use p4kx_archive::Entry;
use p4kx_texture::TextureError;

use crate::buffers::Buffers;
use crate::extract::Ended;
use crate::texture_job::TextureJob;
use crate::texture_parts::Parts;
use crate::write::{EntryWriter, Outcome};
use crate::{ConversionError, EntryFailure, ExtractionError, Texture};

/// How one output of a texture ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    /// The file was written.
    Written,
    /// The file already there was kept under the Conflict policy.
    Kept,
    /// The run stopped.
    Cancelled,
}

/// What converting a texture got done before it ended or failed.
#[derive(Debug, Default)]
struct Done {
    merged: Option<Step>,
    png: Option<Step>,
}

impl Done {
    fn cancelled(&self) -> bool {
        self.merged == Some(Step::Cancelled) || self.png == Some(Step::Cancelled)
    }

    fn outcome(&self) -> Outcome {
        if [self.merged, self.png].contains(&Some(Step::Written)) {
            Outcome::Converted
        } else {
            Outcome::Kept
        }
    }

    const fn merged_in_place(&self) -> bool {
        matches!(self.merged, Some(Step::Written | Step::Kept))
    }
}

/// The texture being converted, with its alpha tail, its header Entry and its number of parts.
pub(crate) struct Source<'j> {
    pub(crate) texture: Texture<'j>,
    pub(crate) header: &'j Entry,
    pub(crate) part_count: u32,
}

impl Source<'_> {
    pub(crate) fn texture_error(&self, error: TextureError) -> ExtractionError {
        ExtractionError::Texture(ConversionError::of_texture(self.texture.name(), error))
    }
}

impl EntryWriter<'_> {
    /// Converts the texture of `job` as the run's texture options say and reports the end of each
    /// of its Entries; a texture that fails is reported and its raw Entries are written instead,
    /// except the header when the merged DDS already took its path.
    pub(crate) fn write_texture(&self, job: &TextureJob<'_>, buffers: &mut Buffers) -> Ended {
        let mut done = Done::default();
        let failure = match self.convert_texture(job, buffers, &mut done) {
            Ok(()) => None,
            Err(_) if self.tracker.archive_file_changed() => {
                done.merged = Some(Step::Cancelled);
                None
            }
            Err(error) => Some(error),
        };
        let raw = self.textures.keep_raw || failure.is_some();
        let failure = failure.filter(|error| !matches!(error, ExtractionError::UnsafePath));
        let mut entries = Vec::with_capacity(job.requested.len() + 1);
        for (position, (index, entry)) in job.entries().enumerate() {
            let is_header = position == 0;
            let outcome = if done.cancelled() {
                self.tracker.ended(entry, 0);
                Outcome::Cancelled
            } else if raw && !(is_header && done.merged_in_place()) {
                match self.write_raw(entry, buffers) {
                    Outcome::Written if failure.is_none() => Outcome::Converted,
                    outcome => outcome,
                }
            } else {
                self.tracker.ended(entry, 0);
                done.outcome()
            };
            entries.push((index, entry.path().to_owned(), outcome));
        }
        buffers.trim();
        let unconverted = failure.map(|error| {
            let (index, header) = job.header;
            let path = header.path().to_owned();
            (index, EntryFailure { path, error })
        });
        Ended::Texture {
            entries,
            unconverted,
        }
    }

    fn convert_texture(
        &self,
        job: &TextureJob<'_>,
        buffers: &mut Buffers,
        done: &mut Done,
    ) -> Result<(), ExtractionError> {
        if self.tracker.should_stop() {
            done.merged = Some(Step::Cancelled);
            return Ok(());
        }
        let (_, header) = job.header;
        let destination = self.destination(header)?;
        let source = self.open_source(job)?;
        let dds = source.texture.header();
        let level = self
            .textures
            .png
            .map(|level| level.min(dds.mip_count().saturating_sub(1)));
        let capture = level
            .filter(|&level| level < source.part_count && dds.format().is_decodable())
            .map(|level| source.part_count - level);
        buffers.data.clear();
        if self.textures.merge {
            let step = self.write_merged(&source, &destination, capture, buffers)?;
            done.merged = Some(step);
            if step == Step::Cancelled {
                return Ok(());
            }
        }
        let Some(level) = level else {
            return Ok(());
        };
        done.png = Some(self.write_pngs(&source, &destination, level, buffers)?);
        Ok(())
    }

    fn open_source<'j>(&'j self, job: &'j TextureJob<'_>) -> Result<Source<'j>, ExtractionError> {
        let (_, header) = job.header;
        let colour = Parts::Archive {
            archive: self.archive,
            header,
            parts: job.parts.clone(),
        };
        let alpha = job.alpha_header.map(|alpha_header| Parts::Archive {
            archive: self.archive,
            header: alpha_header,
            parts: job.alpha_parts.clone(),
        });
        let texture = Texture::from_parts(colour, alpha).map_err(ExtractionError::Texture)?;
        let part_count = texture.part_count().map_err(|error| {
            ExtractionError::Texture(ConversionError::of_texture(texture.name(), error))
        })?;
        Ok(Source {
            texture,
            header,
            part_count,
        })
    }
}
