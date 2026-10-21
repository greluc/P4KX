use std::error::Error;

use p4kx_gltf::{LoadedTexture, TextureLoadError, TextureLoader};
use p4kx_texture::TextureFormat;

use crate::path_index::PathIndex;
use crate::{ConversionError, Texture};

/// The most pixels one texture of a model may hold: 8192 × 8192.
pub const MAX_MODEL_TEXTURE_PIXELS: u64 = 1 << 26;

const DXGI_WITH_ALPHA: &[u32] = &[
    1, 2, 3, 4, 9, 10, 11, 12, 13, 14, 23, 24, 25, 27, 28, 29, 30, 31, 32, 65, 73, 74, 75, 76, 77,
    78, 86, 87, 90, 91, 97, 98, 99, 115,
];

/// Loads the textures of a model's materials from the Entries of an Archive.
pub(crate) struct ArchiveTextures<'i, 'a> {
    pub(crate) index: &'i PathIndex<'a>,
}

impl TextureLoader for ArchiveTextures<'_, '_> {
    fn load(&self, path: &str, mip: u32) -> Result<LoadedTexture, TextureLoadError> {
        let entry = format!("Data\\{}", path.replace('/', "\\"));
        let texture = Texture::from_index(self.index, &entry).map_err(|error| match error {
            ConversionError::UnknownEntry(_) => TextureLoadError::NotFound,
            other => TextureLoadError::Unreadable(describe(&other)),
        })?;
        let header = texture.header();
        if header.surfaces() > 1 || header.depth() > 1 {
            return Err(TextureLoadError::Unreadable(
                "the texture is a cubemap, array or volume, not one image".to_owned(),
            ));
        }
        let level = mip.min(header.mip_count().saturating_sub(1));
        let side = |full: u32| (full >> level).max(1);
        let pixels = u64::from(side(header.width())) * u64::from(side(header.height()));
        if pixels > MAX_MODEL_TEXTURE_PIXELS {
            return Err(TextureLoadError::Unreadable(format!(
                "the texture has {pixels} pixels at this mip, more than the limit of \
                 {MAX_MODEL_TEXTURE_PIXELS}; use a higher --texture-mip"
            )));
        }
        let has_alpha = texture.has_alpha_tail() || format_has_alpha(header.format());
        let image = texture
            .decode_mip(level)
            .map_err(|error| TextureLoadError::Unreadable(describe(&error)))?;
        Ok(LoadedTexture::new(image, has_alpha))
    }
}

/// Returns whether pixels of `format` have an alpha channel that holds data.
fn format_has_alpha(format: TextureFormat) -> bool {
    match format {
        TextureFormat::Dxgi(dxgi) => DXGI_WITH_ALPHA.contains(&dxgi.0),
        TextureFormat::L8A8 => true,
        _ => false,
    }
}

/// Returns the message of `error` and of its sources, separated by colons.
fn describe(error: &dyn Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(inner) = source {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        source = inner.source();
    }
    text
}
