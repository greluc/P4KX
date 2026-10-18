use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_gfx::{Assets, Bitmap, BitmapSource, GfxError, MorphShape, RgbaImage, Shape};
use rayon::prelude::*;

use crate::output_file::{write_error, write_replacing};
use crate::ui_external::external_image;
use crate::{ConversionError, UiMovie};

/// The kind of a definition [`UiMovie::extract_assets`] writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiAssetKind {
    /// A bitmap, written as `bitmap_<id>.png`.
    Bitmap,
    /// A shape, written as `shape_<id>.json`.
    Shape,
    /// A morph shape, written as `morph_shape_<id>.json`.
    MorphShape,
}

impl UiAssetKind {
    /// Returns the name of the kind, which starts its file names.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Bitmap => "bitmap",
            Self::Shape => "shape",
            Self::MorphShape => "morph_shape",
        }
    }

    const fn extension(self) -> &'static str {
        match self {
            Self::Bitmap => "png",
            Self::Shape | Self::MorphShape => "json",
        }
    }
}

/// One definition [`UiMovie::extract_assets`] wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenUiAsset {
    /// The kind of the definition.
    pub kind: UiAssetKind,
    /// The ID of the definition.
    pub id: u16,
    /// The file written.
    pub path: PathBuf,
    /// Its size in bytes.
    pub size: u64,
}

/// One definition [`UiMovie::extract_assets`] could not write.
#[derive(Debug)]
pub struct UiAssetFailure {
    /// The kind of the definition.
    pub kind: UiAssetKind,
    /// The ID of the definition.
    pub id: u16,
    /// Why.
    pub error: ConversionError,
}

/// What [`UiMovie::extract_assets`] did: bitmaps, shapes and morph shapes, each in file order.
#[derive(Debug, Default)]
pub struct UiExtraction {
    /// The definitions written.
    pub written: Vec<WrittenUiAsset>,
    /// The definitions that failed.
    pub failures: Vec<UiAssetFailure>,
}

enum Job<'b, 'c> {
    Bitmap(&'b Bitmap<'c>),
    Shape(&'b Shape),
    MorphShape(&'b MorphShape),
}

impl Job<'_, '_> {
    const fn kind(&self) -> UiAssetKind {
        match self {
            Self::Bitmap(_) => UiAssetKind::Bitmap,
            Self::Shape(_) => UiAssetKind::Shape,
            Self::MorphShape(_) => UiAssetKind::MorphShape,
        }
    }

    const fn id(&self) -> u16 {
        match self {
            Self::Bitmap(bitmap) => bitmap.id,
            Self::Shape(shape) => shape.id,
            Self::MorphShape(morph) => morph.id,
        }
    }
}

impl UiMovie<'_> {
    /// Writes every bitmap of the movie as an 8-bit RGBA PNG and every shape and morph shape as
    /// JSON in twips into `folder`, in parallel, replacing existing files; a definition that fails
    /// leaves no file and does not stop the others. Of several definitions of one kind with the
    /// same ID only the first is written.
    ///
    /// Embedded and lossless bitmaps are decoded from the movie; an external image is decoded
    /// from the DDS texture its file name names, relative to the movie, with the extension `.dds`
    /// tried as well; a sub-image is cut from the bitmap it refers to.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::Write`] when `folder` cannot be created.
    pub fn extract_assets(&self, folder: &Path) -> Result<UiExtraction, ConversionError> {
        fs::create_dir_all(folder).map_err(|source| write_error(folder, source))?;
        let assets = self.movie().assets();
        let mut seen = HashSet::new();
        let jobs: Vec<Job<'_, '_>> = assets
            .bitmaps
            .iter()
            .map(Job::Bitmap)
            .chain(assets.shapes.iter().map(Job::Shape))
            .chain(assets.morph_shapes.iter().map(Job::MorphShape))
            .filter(|job| seen.insert((job.kind(), job.id())))
            .collect();
        let outcomes: Vec<(UiAssetKind, u16, Result<WrittenUiAsset, ConversionError>)> = jobs
            .par_iter()
            .map(|job| (job.kind(), job.id(), self.write_job(&assets, job, folder)))
            .collect();
        let mut extraction = UiExtraction::default();
        for (kind, id, outcome) in outcomes {
            match outcome {
                Ok(written) => extraction.written.push(written),
                Err(error) => extraction.failures.push(UiAssetFailure { kind, id, error }),
            }
        }
        Ok(extraction)
    }

    fn write_job(
        &self,
        assets: &Assets<'_>,
        job: &Job<'_, '_>,
        folder: &Path,
    ) -> Result<WrittenUiAsset, ConversionError> {
        let kind = job.kind();
        let id = job.id();
        let path = folder.join(format!("{}_{id}.{}", kind.name(), kind.extension()));
        let image = match job {
            Job::Bitmap(bitmap) => Some(self.bitmap_image(assets, bitmap)?),
            Job::Shape(_) | Job::MorphShape(_) => None,
        };
        let gfx_error = |source| match source {
            GfxError::Write(source) => write_error(&path, source),
            source => self.asset_error(id, source),
        };
        write_replacing(&path, |writer| {
            match job {
                Job::Bitmap(_) => image.map_or(Ok(()), |image| image.write_png(writer)),
                Job::Shape(shape) => shape.write_json(writer),
                Job::MorphShape(morph) => morph.write_json(writer),
            }
            .map_err(gfx_error)
        })?;
        let size = fs::metadata(&path)
            .map_err(|source| write_error(&path, source))?
            .len();
        Ok(WrittenUiAsset {
            kind,
            id,
            path,
            size,
        })
    }

    pub(crate) fn asset_error(&self, id: u16, source: GfxError) -> ConversionError {
        ConversionError::UiAsset {
            movie: self.name().to_owned(),
            id,
            source,
        }
    }

    fn bitmap_image(
        &self,
        assets: &Assets<'_>,
        bitmap: &Bitmap<'_>,
    ) -> Result<RgbaImage, ConversionError> {
        match &bitmap.source {
            BitmapSource::External(image) => external_image(self, bitmap.id, image),
            BitmapSource::SubImage(sub) => {
                let missing = || ConversionError::UiSubImage {
                    movie: self.name().to_owned(),
                    id: bitmap.id,
                    image_id: sub.image_id,
                };
                let base = assets
                    .bitmaps
                    .iter()
                    .find(|base| {
                        base.id == sub.image_id && !matches!(base.source, BitmapSource::SubImage(_))
                    })
                    .ok_or_else(missing)?;
                self.bitmap_image(assets, base)?
                    .crop(
                        (u32::from(sub.x1), u32::from(sub.y1)),
                        (u32::from(sub.x2), u32::from(sub.y2)),
                    )
                    .ok_or_else(missing)
            }
            BitmapSource::Lossless(_) | BitmapSource::Jpeg(_) => assets
                .decode_bitmap(bitmap)
                .map_err(|source| self.asset_error(bitmap.id, source)),
        }
    }
}
