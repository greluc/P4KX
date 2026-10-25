// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{CUBE_FACE_NAMES, DdsHeader, Texture};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::container::Place;
use crate::error::ToolError;

/// The side length a preview fits when a call gives none.
pub(crate) const DEFAULT_SIDE: u32 = 512;
/// The largest side length a call may ask for.
pub(crate) const MAX_SIDE: u32 = 2048;
/// The most pixels of a preview.
pub(crate) const MAX_PIXELS: u64 = 2048 * 2048;
/// The largest PNG a call returns, in bytes.
pub(crate) const MAX_PNG_LEN: usize = 1 << 20;
/// The most bytes of texture data one preview reads.
const MAX_MIP_LEN: u64 = 128 << 20;

/// The arguments of `image_preview`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImageParams {
    /// A Channel name such as `LIVE`, or the absolute path of a `.p4k` file; omit it for the
    /// Archive the server was started with.
    pub(crate) archive: Option<String>,
    /// The Entry path of the texture header, e.g. `Data/Textures/ui/logo.dds`; its parts
    /// `.dds.1` and its alpha tail `.dds.a` are found beside it. An Entry inside a container is
    /// `outer.socpak::path`.
    pub(crate) path: String,
    /// The mip level, 0 being the largest; omit it for the largest level that fits `max_side`.
    pub(crate) mip: Option<u32>,
    /// The side length in pixels the image fits when `mip` is omitted, 512 by default, at most
    /// 2048.
    pub(crate) max_side: Option<u32>,
    /// The face of a cubemap to show, `posx`, `negx`, `posy`, `negy`, `posz` or `negz`, or its
    /// number 0 to 5; omit it to show all six faces as a cross.
    pub(crate) face: Option<String>,
}

/// What an image preview shows; the PNG itself is the image content block of the result.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ImageResult {
    /// The path of the Archive.
    pub(crate) archive: String,
    /// The Entry path of the texture header.
    pub(crate) path: String,
    /// The pixel format of the texture, e.g. `BC7_UNORM_SRGB`.
    pub(crate) format: String,
    /// The width of the texture at mip 0 in pixels.
    pub(crate) texture_width: u32,
    /// The height of the texture at mip 0 in pixels.
    pub(crate) texture_height: u32,
    /// The number of mip levels.
    pub(crate) mip_count: u32,
    /// The mip level shown.
    pub(crate) mip: u32,
    /// The width of the image shown in pixels.
    pub(crate) width: u32,
    /// The height of the image shown in pixels.
    pub(crate) height: u32,
    /// Whether the texture is a cubemap.
    pub(crate) cubemap: bool,
    /// The face shown; absent when the image is the whole texture, a cubemap as a cross.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) face: Option<&'static str>,
    /// The number of array elements.
    pub(crate) array_size: u32,
    /// The number of volume slices at mip 0.
    pub(crate) depth: u32,
    /// Whether an alpha or smoothness tail was merged into the alpha channel.
    pub(crate) alpha_tail: bool,
    /// The media type of the image content block: always `image/png`.
    pub(crate) mime_type: &'static str,
    /// The size of the PNG in bytes.
    pub(crate) png_bytes: usize,
}

/// Returns the preview of the texture whose header is `path` in `place` and its PNG.
pub(crate) fn preview(
    place: &Place<'_>,
    params: &ImageParams,
    path: &str,
) -> Result<(ImageResult, Vec<u8>), ToolError> {
    let index = place.index();
    let texture = Texture::from_index(&index, path)
        .map_err(|error| ToolError::converted(path, "a texture", error))?;
    let header = texture.header();
    let face = params
        .face
        .as_deref()
        .map(|text| face_of(text, header))
        .transpose()?;
    let side = params.max_side.unwrap_or(DEFAULT_SIDE).clamp(1, MAX_SIDE);
    let level = level_of(params.mip, side, face.is_some(), header)?;
    let [width, height] = shown_size(header, level, face.is_some());
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(refused(format!(
            "mip {level} of {path} is {width}x{height} pixels, more than the {MAX_PIXELS} a preview shows; ask for a higher mip or a smaller max_side"
        )));
    }
    if header
        .mip_size(level)
        .is_some_and(|size| size > MAX_MIP_LEN)
    {
        return Err(refused(format!(
            "mip {level} of {path} holds more than {MAX_MIP_LEN} bytes; ask for a higher mip"
        )));
    }
    let shown = texture
        .preview_png(level, face)
        .map_err(|error| ToolError::converted(path, "a texture", error))?;
    if shown.png.len() > MAX_PNG_LEN {
        return Err(refused(format!(
            "the PNG of mip {level} of {path} is {} bytes, more than the {MAX_PNG_LEN} a preview returns; ask for a higher mip",
            shown.png.len()
        )));
    }
    let result = ImageResult {
        archive: place.archive().path().display().to_string(),
        path: place.shown(texture.name()),
        format: header.format().to_string(),
        texture_width: header.width(),
        texture_height: header.height(),
        mip_count: header.mip_count(),
        mip: shown.level,
        width: shown.width,
        height: shown.height,
        cubemap: header.is_cubemap(),
        face: face.and_then(|face| CUBE_FACE_NAMES.get(face).copied()),
        array_size: header.array_size(),
        depth: header.depth(),
        alpha_tail: texture.has_alpha_tail(),
        mime_type: "image/png",
        png_bytes: shown.png.len(),
    };
    Ok((result, shown.png))
}

const fn refused(message: String) -> ToolError {
    ToolError::Refused(message)
}

fn face_of(text: &str, header: &DdsHeader) -> Result<usize, ToolError> {
    if !header.is_cubemap() {
        return Err(refused("the texture is no cubemap; omit face".to_owned()));
    }
    let text = text.trim();
    CUBE_FACE_NAMES
        .iter()
        .position(|name| name.eq_ignore_ascii_case(text))
        .or_else(|| text.parse::<usize>().ok().filter(|&face| face < 6))
        .ok_or_else(|| {
            refused(format!(
                "`{text}` is no cubemap face; use posx, negx, posy, negy, posz, negz or 0 to 5"
            ))
        })
}

fn shown_size(header: &DdsHeader, level: u32, face: bool) -> [u32; 2] {
    if face {
        let side = |full: u32| full.checked_shr(level).unwrap_or(0).max(1);
        [side(header.width()), side(header.height())]
    } else {
        header.preview_size(level).unwrap_or([u32::MAX; 2])
    }
}

fn level_of(
    requested: Option<u32>,
    side: u32,
    face: bool,
    header: &DdsHeader,
) -> Result<u32, ToolError> {
    let count = header.mip_count();
    if let Some(level) = requested {
        return if level < count {
            Ok(level)
        } else {
            Err(refused(format!(
                "the texture has {count} mip levels; mip {level} does not exist"
            )))
        };
    }
    Ok((0..count)
        .find(|&level| {
            shown_size(header, level, face)
                .iter()
                .all(|&size| size <= side)
        })
        .unwrap_or_else(|| count.saturating_sub(1)))
}
