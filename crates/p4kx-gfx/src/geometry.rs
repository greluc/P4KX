// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde::Serialize;

/// A rectangle in twips, a twentieth of a pixel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
pub struct Rect {
    /// The left edge.
    pub x_min: i32,
    /// The right edge.
    pub x_max: i32,
    /// The top edge.
    pub y_min: i32,
    /// The bottom edge.
    pub y_max: i32,
}

impl Rect {
    /// Returns the width in twips.
    #[must_use]
    pub const fn width(&self) -> i64 {
        self.x_max as i64 - self.x_min as i64
    }

    /// Returns the height in twips.
    #[must_use]
    pub const fn height(&self) -> i64 {
        self.y_max as i64 - self.y_min as i64
    }
}

/// An affine transform; the translation is in twips.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Matrix {
    /// The horizontal scale.
    pub scale_x: f32,
    /// The vertical scale.
    pub scale_y: f32,
    /// The first rotation and skew factor.
    pub rotate_skew0: f32,
    /// The second rotation and skew factor.
    pub rotate_skew1: f32,
    /// The horizontal translation.
    pub translate_x: i32,
    /// The vertical translation.
    pub translate_y: i32,
}

impl Matrix {
    /// The transform that changes nothing.
    pub const IDENTITY: Self = Self {
        scale_x: 1.0,
        scale_y: 1.0,
        rotate_skew0: 0.0,
        rotate_skew1: 0.0,
        translate_x: 0,
        translate_y: 0,
    };
}

/// A colour with straight alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Rgba {
    /// Red.
    pub red: u8,
    /// Green.
    pub green: u8,
    /// Blue.
    pub blue: u8,
    /// Opacity, 255 being opaque.
    pub alpha: u8,
}

/// A position or a movement in twips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Point {
    /// The horizontal coordinate.
    pub x: i32,
    /// The vertical coordinate.
    pub y: i32,
}
