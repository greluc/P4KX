// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gfx::Affine;

use crate::canvas::{Canvas, Coordinates};
use crate::paint::Area;
use crate::widget::CanvasSizing;

/// How a canvas is laid out in the box it is drawn in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Fit {
    /// The box the widgets of the canvas are laid out in, in the units of the canvas.
    pub(crate) layout: Area,
    /// Maps the units of the canvas into the box, with the scale it applies, when they differ.
    pub(crate) scaled: Option<(Affine, f32)>,
    /// Whether the origin of the root widgets is the centre of the box.
    pub(crate) centred: bool,
}

fn usable(scale: f32) -> bool {
    scale.is_finite() && scale > 0.0
}

const fn scaling(scale: f32, x: f32, y: f32) -> (Affine, f32) {
    (
        Affine {
            a: scale,
            b: 0.0,
            c: 0.0,
            d: scale,
            tx: x,
            ty: y,
        },
        scale,
    )
}

/// Decides how the units of `canvas` relate to the pixels of `frame`.
///
/// A raw canvas whose root widget is a few units large (a world-space canvas, such as a HUD) is
/// scaled to fit that root into the frame; a canvas that a canvas widget scales
/// ([`CanvasSizing::Scale`]) is laid out at its own size and scaled to fit the widget; the
/// coordinate methods `aspectOverridesWidth` and `aspectOverridesHeight` take the width from the
/// height and the height from the width by the aspect of the canvas.
pub(crate) fn fit(canvas: &Canvas, frame: Area, sizing: CanvasSizing) -> Fit {
    let plain = Fit {
        layout: frame,
        scaled: None,
        centred: false,
    };
    if let Some([width, height]) = canvas.world_root {
        let scale = (frame.width / width).min(frame.height / height);
        if usable(scale) {
            return Fit {
                layout: Area {
                    x: 0.0,
                    y: 0.0,
                    width: frame.width / scale,
                    height: frame.height / scale,
                },
                scaled: Some(scaling(scale, frame.x, frame.y)),
                centred: true,
            };
        }
    }
    let [width, height] = canvas.size;
    let valid = width > 0.0 && height > 0.0;
    if sizing == CanvasSizing::Scale && valid {
        let scale = (frame.width / width).min(frame.height / height);
        if usable(scale) {
            return Fit {
                layout: Area {
                    x: 0.0,
                    y: 0.0,
                    width,
                    height,
                },
                scaled: Some(scaling(
                    scale,
                    frame.x + (-width).mul_add(scale, frame.width) / 2.0,
                    frame.y + (-height).mul_add(scale, frame.height) / 2.0,
                )),
                centred: false,
            };
        }
    }
    match canvas.coordinates {
        Coordinates::AspectOverridesWidth if valid => Fit {
            layout: Area {
                width: frame.height * width / height,
                ..frame
            },
            ..plain
        },
        Coordinates::AspectOverridesHeight if valid => Fit {
            layout: Area {
                height: frame.width * height / width,
                ..frame
            },
            ..plain
        },
        _ => plain,
    }
}
