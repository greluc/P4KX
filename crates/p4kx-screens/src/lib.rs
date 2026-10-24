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
//! in the [`Report`] of the [`Composed`] scene. `ActionScript` is never run and no texture or text
//! is drawn.

mod assets;
mod canvas;
mod colour;
mod compose;
mod context;
mod elements;
mod error;
mod flex;
mod inspect;
mod layout;
mod look;
mod paint;
mod reader;
mod report;
mod screens;
mod style;
mod walk;
mod widget;

pub use assets::ScreenAssets;
pub use compose::{Composed, ScreenComposer};
pub use error::ScreenError;
pub use inspect::ElementCount;
pub use report::{Report, Unsupported};
pub use screens::{EntityScreens, MovieUse, Screen};
