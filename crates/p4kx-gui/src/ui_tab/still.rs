// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::gfx::Localise;
use p4kx_extraction::{ArchiveImages, CancelToken, DataForge, ScreenTexts, decode_line_breaks};
use p4kx_screens::{
    ImageSources, MAX_SVG_BYTES, ScreenComposer, ScreenError, Still, StillRequest, TextSources,
};

use super::render::{Fail, reason};
use crate::EntryIndex;

/// Draws `request` of `forge` with the fonts and texts of `texts` and the textures and vector
/// graphics of the Archive of `index`, stopping when `cancel` is cancelled.
///
/// Returns the still and why a source of its texts was left out.
pub(super) fn draw(
    index: &EntryIndex,
    forge: &DataForge<'_>,
    texts: &ScreenTexts,
    request: &StillRequest<'_>,
    cancel: &CancelToken,
) -> Result<(Still, Vec<String>), Fail> {
    let mut warnings = texts.warnings().to_vec();
    let ini = texts.parsed(&mut warnings);
    let lookup = |key: &str| {
        ini.as_ref()?
            .resolve(key)
            .map(|text| decode_line_breaks(text).into_owned())
    };
    let images = ArchiveImages::new(index.archive());
    let read_texture = |path: &str, side: u32, pixels: u64| {
        images
            .texture(path, side, pixels)
            .map_err(|error| crate::model::describe(&error))
    };
    let read_vector = |path: &str| {
        images
            .vector(path, MAX_SVG_BYTES as u64)
            .map_err(|error| crate::model::describe(&error))
    };
    let text_sources = texts.fonts().map(|fonts| TextSources {
        fonts,
        localise: ini.as_ref().map(|_| &lookup as Localise<'_>),
    });
    let image_sources = ImageSources {
        texture: &read_texture,
        vector: &read_vector,
    };
    let still = Still::render_while(forge, text_sources, Some(image_sources), request, &|| {
        cancel.is_cancelled()
    })
    .map_err(|error| match error {
        ScreenError::Stopped => Fail::Stopped,
        other => reason(&other),
    })?;
    Ok((still, warnings))
}

/// Returns the Record path of the font movie the screens of `forge` draw their texts with.
pub(super) fn font_file(forge: &DataForge<'_>) -> String {
    ScreenComposer::new(forge).font_file()
}

/// Reads the fonts and, for `language`, the texts the screens of `forge` are drawn with.
pub(super) fn load_texts(
    index: &EntryIndex,
    forge: &DataForge<'_>,
    language: Option<crate::TextLanguage>,
) -> ScreenTexts {
    ScreenTexts::load(index.archive(), &font_file(forge), language)
}
