// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::gfx::{Background, CanvasSize, RasterOptions, Rgba};

use crate::UiText;

/// The size of the picture a UI file or a screen is drawn in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum UiSize {
    /// One pixel per pixel of the stage or the canvas.
    #[default]
    Stage,
    /// 1024 pixels wide, the height keeping the aspect ratio.
    Wide1024,
    /// 2048 pixels wide, the height keeping the aspect ratio.
    Wide2048,
    /// 1920 × 1080 pixels, the stage scaled to fit and centred.
    FullHd,
}

impl UiSize {
    /// Every size, in the order the tab offers them.
    pub const ALL: [Self; 4] = [Self::Stage, Self::Wide1024, Self::Wide2048, Self::FullHd];

    /// Returns the size `p4kx ui render --size` would be given.
    #[must_use]
    pub const fn canvas(self) -> CanvasSize {
        match self {
            Self::Stage => CanvasSize::Stage,
            Self::Wide1024 => CanvasSize::Width(1024),
            Self::Wide2048 => CanvasSize::Width(2048),
            Self::FullHd => CanvasSize::Fit {
                width: 1920,
                height: 1080,
            },
        }
    }

    /// Returns the label of the size.
    #[must_use]
    pub const fn text(self) -> UiText {
        match self {
            Self::Stage => UiText::SizeStage,
            Self::Wide1024 => UiText::SizeWide1024,
            Self::Wide2048 => UiText::SizeWide2048,
            Self::FullHd => UiText::SizeFullHd,
        }
    }
}

/// What lies under the picture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum UiBackground {
    /// Nothing.
    #[default]
    Transparent,
    /// The background colour the movie sets.
    Movie,
    /// Opaque black.
    Black,
    /// Opaque white.
    White,
}

impl UiBackground {
    /// Every background, in the order the tab offers them.
    pub const ALL: [Self; 4] = [Self::Transparent, Self::Movie, Self::Black, Self::White];

    /// Returns the background `p4kx ui render --background` would be given.
    #[must_use]
    pub const fn background(self) -> Background {
        match self {
            Self::Transparent => Background::Transparent,
            Self::Movie => Background::Movie,
            Self::Black => Background::Color(Rgba {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 255,
            }),
            Self::White => Background::Color(Rgba {
                red: 255,
                green: 255,
                blue: 255,
                alpha: 255,
            }),
        }
    }

    /// Returns the label of the background.
    #[must_use]
    pub const fn text(self) -> UiText {
        match self {
            Self::Transparent => UiText::BackgroundTransparent,
            Self::Movie => UiText::BackgroundMovie,
            Self::Black => UiText::BackgroundBlack,
            Self::White => UiText::BackgroundWhite,
        }
    }
}

/// The file format an export writes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum UiFormat {
    /// An 8-bit RGBA PNG.
    #[default]
    Png,
    /// An SVG document.
    Svg,
}

impl UiFormat {
    /// Every format, in the order the tab offers them.
    pub const ALL: [Self; 2] = [Self::Png, Self::Svg];

    /// Returns the file extension.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
        }
    }

    /// Returns the label of the format.
    #[must_use]
    pub const fn text(self) -> UiText {
        match self {
            Self::Png => UiText::FormatPng,
            Self::Svg => UiText::FormatSvg,
        }
    }
}

/// The choices of how the tab draws and exports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct UiSettings {
    /// The size of the picture.
    pub size: UiSize,
    /// What lies under it.
    pub background: UiBackground,
    /// The format an export writes.
    pub format: UiFormat,
}

impl UiSettings {
    /// Returns the options the picture is rasterised with.
    #[must_use]
    pub const fn raster(&self) -> RasterOptions {
        RasterOptions {
            size: self.size.canvas(),
            background: self.background.background(),
        }
    }
}
