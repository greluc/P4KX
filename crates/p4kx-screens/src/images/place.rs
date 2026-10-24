// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gfx::{
    Affine, BitmapPaint, ColorTransform, LayerKind, Origin, Paint, PathCommand, SceneItem,
    ScenePath, ScenePoint, SceneShape, ShapeLayer,
};

use crate::paint::{Area, Painter, TWIPS};

fn corner(x: f32, y: f32) -> ScenePoint {
    ScenePoint {
        x: x * TWIPS,
        y: y * TWIPS,
    }
}

impl Painter {
    /// Adds a shape of `layers` that items can place, returning its index.
    pub(crate) fn add_shape(&mut self, layers: Vec<ShapeLayer>) -> usize {
        let index = self.shapes.len();
        let id = u16::try_from(index % usize::from(u16::MAX)).unwrap_or(0);
        self.shapes.push(SceneShape { id, layers });
        index
    }

    /// Places the shape `shape` with `transform` and `colours`.
    pub(crate) fn place(&mut self, shape: usize, transform: Affine, colours: ColorTransform) {
        if self.is_full() {
            return;
        }
        let id = self.shapes.get(shape).map_or(0, |shape| shape.id);
        self.items.push(SceneItem {
            shape,
            transform,
            color_transform: colours,
            mask: None,
            origin: Origin {
                id,
                depth: 0,
                sprite: None,
            },
        });
    }

    /// Draws the bitmap `bitmap`, whose whole image covers `image`, inside `clip`.
    pub(crate) fn bitmap(
        &mut self,
        bitmap: u16,
        size: (u32, u32),
        image: Area,
        clip: Area,
        flip: [bool; 2],
        colours: ColorTransform,
    ) {
        if clip.is_empty() || image.is_empty() || self.is_full() {
            return;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "texture sides are far below 2^24"
        )]
        let (width, height) = (size.0 as f32, size.1 as f32);
        let (scale_x, scale_y) = (image.width / width * TWIPS, image.height / height * TWIPS);
        let (a, tx) = if flip[0] {
            (-scale_x, (image.x + image.width) * TWIPS)
        } else {
            (scale_x, image.x * TWIPS)
        };
        let (d, ty) = if flip[1] {
            (-scale_y, (image.y + image.height) * TWIPS)
        } else {
            (scale_y, image.y * TWIPS)
        };
        let (left, top) = (clip.x, clip.y);
        let (right, bottom) = (clip.x + clip.width, clip.y + clip.height);
        let path = ScenePath {
            commands: vec![
                PathCommand::MoveTo(corner(left, top)),
                PathCommand::LineTo(corner(right, top)),
                PathCommand::LineTo(corner(right, bottom)),
                PathCommand::LineTo(corner(left, bottom)),
                PathCommand::Close,
            ],
        };
        let shape = self.add_shape(vec![ShapeLayer {
            path,
            paint: Paint::Bitmap(BitmapPaint {
                bitmap_id: bitmap,
                transform: Affine {
                    a,
                    b: 0.0,
                    c: 0.0,
                    d,
                    tx,
                    ty,
                },
                repeating: false,
                smoothed: true,
            }),
            kind: LayerKind::Fill,
        }]);
        self.place(shape, Affine::IDENTITY, colours);
    }
}
