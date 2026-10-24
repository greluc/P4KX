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
//! Layout follows the sizing, bounds, margin, padding, anchor, pivot, turn and scale of each
//! widget and the flex, stack and grid containers that arrange children; clones and lists repeat
//! the library templates of a canvas, overflow clips, and raw canvases count in pixels of their
//! own size (ADR-0106). Every widget type has its own module under `elements`; a type without a
//! renderer, and every feature the layout or the painters approximate, is listed in the
//! [`Report`] of the [`Composed`] scene. `ActionScript` is never run and no texture is drawn.
//!
//! Texts are drawn as glyph outlines of the fonts in [`TextSources`], which a caller reads from
//! `fonts_en.gfx`: the font style of a widget and the cascade pick the font, size, letter and line
//! spacing, alignment and case, localisation keys resolve in the language of the caller, and a
//! character without a glyph is an empty box. Without [`TextSources`] text widgets are listed.

mod assets;
mod bindings;
mod boxes;
mod canvas;
mod colour;
mod compose;
mod context;
mod corners;
mod elements;
mod error;
mod fit;
mod flex;
mod font_file;
mod geometry;
mod glyphs;
mod inspect;
mod layout;
mod limits;
mod look;
mod paint;
mod paint_state;
mod reader;
mod report;
mod screens;
mod statics;
mod style;
mod text;
mod transform;
mod walk;
mod walk_canvas;
mod walk_instance;
mod walk_measure;
mod widget;

pub use assets::ScreenAssets;
pub use compose::{Composed, ScreenComposer, TextSources};
pub use error::ScreenError;
pub use inspect::ElementCount;
pub use limits::Limits;
pub use report::{Report, Unsupported};
pub use screens::{EntityScreens, MovieUse, Screen};
