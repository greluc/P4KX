// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Screens of Star Citizen Entities: the `BuildingBlocks` canvases their UI components name, read
//! from the DataForge Records and rendered as stills through the GFx renderer (ADR-0089).
//!
//! [`ScreenComposer::screens_of`] lists the screens of an Entity and of everything in its
//! resolved Loadout with the UI assets each uses. [`ScreenComposer::compose`] lays a canvas out
//! and draws its widgets into a [`Composed`] scene of the canvas size, which
//! [`Composed::rasterize`] draws as an image and [`Composed::write_svg`] writes as SVG.
//! The colour slots of a widget take their colours from the brand style of the canvas.
//!
//! Layout follows the sizing, margin, padding and position of each widget and the flex
//! containers that arrange children. Every widget type has its own module under `elements`; a
//! type without a renderer, and every feature the layout or the painters approximate, is listed
//! in the [`Report`] of the [`Composed`] scene. `ActionScript` is never run.
//!
//! Texts are drawn as glyph outlines of the fonts in [`TextSources`], which a caller reads from
//! `fonts_en.gfx`: the font style of a widget and the cascade pick the font, size, letter and line
//! spacing, alignment and case, localisation keys resolve in the language of the caller, and a
//! character without a glyph is an empty box. Without [`TextSources`] text widgets are listed.
//!
//! Textures and vector graphics are read through [`ImageSources`]: image widgets draw their
//! texture as a bitmap fill with the scaling method, flips and colour of the widget, and icons,
//! custom shapes and images that name an SVG draw its paths as shapes, fitted to the widget, with
//! a colour overlay or palette slots and an optional nine-slice. The files are untrusted and
//! bounded by [`Limits`]; one that is missing or unreadable is listed and its widget skipped.
//! Without [`ImageSources`] these widgets are listed.

mod assets;
mod bindings;
mod canvas;
mod colour;
mod compose;
mod context;
mod elements;
mod error;
mod flex;
mod font_file;
mod glyphs;
mod images;
mod inspect;
mod layout;
mod limits;
mod look;
mod paint;
mod picture;
mod reader;
mod report;
mod screens;
mod style;
mod svg;
mod text;
mod walk;
mod widget;

pub use assets::ScreenAssets;
pub use compose::{Composed, ScreenComposer, TextSources};
pub use error::ScreenError;
pub use images::{ImageSources, LoadTexture, LoadVector};
pub use inspect::ElementCount;
pub use limits::Limits;
pub use report::{Report, Unsupported};
pub use screens::{EntityScreens, MovieUse, Screen};
pub use svg::MAX_SVG_BYTES;
