// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod fit;
mod place;

use std::borrow::Cow;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use p4kx_gfx::RgbaImage;
use p4kx_texture::RgbaImage as Decoded;

pub(crate) use self::fit::{texture_rect, vector_layers, vector_transform};
use crate::limits::Limits;
use crate::svg::{self, Vector};

/// Reads and decodes the texture at a path of the Archive.
///
/// The second argument is the side in pixels that the screen is at most drawn at, from which an
/// implementation can pick a mip level to decode; the result is 8-bit RGBA.
pub type LoadTexture<'a> = &'a dyn Fn(&str, u32) -> Result<Decoded, String>;

/// Reads the bytes of a vector graphic at a path of the Archive, at most
/// [`MAX_SVG_BYTES`](crate::MAX_SVG_BYTES) of them.
pub type LoadVector<'a> = &'a dyn Fn(&str) -> Result<Vec<u8>, String>;

/// Where the textures and vector graphics of screens are read from.
#[derive(Clone, Copy)]
pub struct ImageSources<'a> {
    /// Reads and decodes a texture, whose path is the `.dds` file the Archive holds.
    pub texture: LoadTexture<'a>,
    /// Reads a `.svg` file.
    pub vector: LoadVector<'a>,
}

const MAX_TEXTURES: usize = 1 << 10;
const MAX_TEXTURE_PIXELS: u64 = 1 << 25;
const MAX_VECTORS: usize = 1 << 10;

/// A decoded texture of the scene.
#[derive(Debug)]
pub(crate) struct Texture {
    pub(crate) id: u16,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// A parsed vector graphic with the shape it was placed as.
#[derive(Debug)]
pub(crate) struct VectorEntry {
    pub(crate) vector: Vector,
    pub(crate) shape: Cell<Option<usize>>,
    pub(crate) reported: Cell<bool>,
}

/// The textures and vector graphics of one composition, read once per path.
#[derive(Default)]
pub(crate) struct Images<'t> {
    sources: Option<ImageSources<'t>>,
    side: u32,
    limits: Limits,
    textures: HashMap<String, Result<Rc<Texture>, String>>,
    bitmaps: HashMap<u16, RgbaImage>,
    pixels: u64,
    vectors: HashMap<String, Result<Rc<VectorEntry>, String>>,
    commands: usize,
}

impl<'t> Images<'t> {
    pub(crate) fn new(sources: ImageSources<'t>, side: u32, limits: Limits) -> Self {
        Self {
            sources: Some(sources),
            side,
            limits,
            ..Self::default()
        }
    }

    pub(crate) const fn is_enabled(&self) -> bool {
        self.sources.is_some()
    }

    pub(crate) fn into_bitmaps(self) -> HashMap<u16, RgbaImage> {
        self.bitmaps
    }

    pub(crate) fn texture(&mut self, path: &str) -> Result<Rc<Texture>, String> {
        let key = path.to_ascii_lowercase();
        if let Some(known) = self.textures.get(&key) {
            return known.clone();
        }
        let loaded = self.load_texture(path);
        self.textures.insert(key, loaded.clone());
        loaded
    }

    fn load_texture(&mut self, path: &str) -> Result<Rc<Texture>, String> {
        let sources = self.sources.ok_or("no image sources were given")?;
        if self.textures.len() >= MAX_TEXTURES {
            return Err(format!("the screen uses more than {MAX_TEXTURES} textures"));
        }
        let image = (sources.texture)(path, self.side)?;
        let (width, height) = (image.width(), image.height());
        let pixels = u64::from(width) * u64::from(height);
        if pixels == 0 || pixels > MAX_TEXTURE_PIXELS.min(self.limits.texture_pixels) {
            return Err(format!("the texture has {width} x {height} pixels"));
        }
        if self.pixels.saturating_add(pixels) > self.limits.texture_pixels {
            return Err("the textures of the screen exceed the pixel budget".to_owned());
        }
        let image = RgbaImage::from_pixels(width, height, image.into_pixels())
            .map_err(|error| error.to_string())?;
        let id = u16::try_from(self.bitmaps.len() + 1).map_err(|_| "too many textures")?;
        self.pixels += pixels;
        self.bitmaps.insert(id, image);
        Ok(Rc::new(Texture { id, width, height }))
    }

    /// Charges the path commands of a copy of a vector graphic against the budget of the screen.
    pub(crate) const fn charge_copy(&mut self, commands: usize) -> bool {
        let total = self.commands.saturating_add(commands);
        if total > self.limits.vector_commands {
            return false;
        }
        self.commands = total;
        true
    }

    pub(crate) fn vector(&mut self, path: &str) -> Result<Rc<VectorEntry>, String> {
        let key = path.to_ascii_lowercase();
        if let Some(known) = self.vectors.get(&key) {
            return known.clone();
        }
        let loaded = self.load_vector(path);
        self.vectors.insert(key, loaded.clone());
        loaded
    }

    fn load_vector(&mut self, path: &str) -> Result<Rc<VectorEntry>, String> {
        let sources = self.sources.ok_or("no image sources were given")?;
        if self.vectors.len() >= MAX_VECTORS {
            return Err(format!(
                "the screen uses more than {MAX_VECTORS} vector graphics"
            ));
        }
        let bytes = (sources.vector)(path)?;
        let vector = svg::parse(&bytes).map_err(|error| error.to_string())?;
        if self.commands.saturating_add(vector.commands) > self.limits.vector_commands {
            return Err("the vector graphics of the screen exceed the path budget".to_owned());
        }
        self.commands += vector.commands;
        Ok(Rc::new(VectorEntry {
            vector,
            shape: Cell::new(None),
            reported: Cell::new(false),
        }))
    }
}

impl fmt::Debug for Images<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Images")
            .field("enabled", &self.sources.is_some())
            .field("textures", &self.textures.len())
            .field("vectors", &self.vectors.len())
            .finish_non_exhaustive()
    }
}

/// Returns the path of the file the Archive holds for a texture path of a Record: a `.tif` is a
/// `.dds`.
pub(crate) fn archive_texture_path(path: &str) -> Cow<'_, str> {
    match path.rsplit_once('.') {
        Some((base, extension))
            if extension.eq_ignore_ascii_case("tif") || extension.eq_ignore_ascii_case("tiff") =>
        {
            Cow::Owned(format!("{base}.dds"))
        }
        _ => Cow::Borrowed(path),
    }
}
