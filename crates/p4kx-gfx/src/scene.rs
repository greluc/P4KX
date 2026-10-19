use crate::geometry::{Matrix, Rect, Rgba};
use crate::place::ColorTransform;
use crate::scene_report::Skipped;
use crate::style::GradientStop;

/// The most shape layers a scene draws, summed over every placement.
pub const MAX_SCENE_PATHS: usize = 1 << 20;

/// The most placements a scene visits, sprites and their contents included.
pub const MAX_SCENE_PLACEMENTS: usize = 1 << 18;

/// The most masks a scene holds.
pub const MAX_SCENE_MASKS: usize = 256;

/// The most stops a gradient may have, as many as a SWF gradient can hold.
pub const MAX_GRADIENT_STOPS: usize = 15;

/// A frame of a UI movie as drawing instructions in twips, independent of any backend.
///
/// It holds each shape once, its placements with transforms, colour transforms and masks, and
/// what could not be drawn.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    /// The stage of the movie.
    pub stage: Rect,
    /// The background colour the movie sets.
    pub background: Option<Rgba>,
    /// The shapes the placements draw, each converted once.
    pub shapes: Vec<SceneShape>,
    /// The placements of shapes from bottom to top.
    pub items: Vec<SceneItem>,
    /// The masks items are clipped by; a mask comes after its parent.
    pub masks: Vec<SceneMask>,
    /// What was skipped or drawn without an effect it asks for.
    pub skipped: Vec<Skipped>,
}

impl Scene {
    /// Returns the IDs of the bitmaps the shapes of the scene fill with, each once, ascending.
    #[must_use]
    pub fn bitmap_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = self
            .shapes
            .iter()
            .flat_map(|shape| &shape.layers)
            .filter_map(|layer| match &layer.paint {
                Paint::Bitmap(bitmap) => Some(bitmap.bitmap_id),
                Paint::Solid(_) | Paint::Gradient(_) => None,
            })
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }
}

/// Where an item or a skipped element comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Origin {
    /// The ID of the definition placed.
    pub id: u16,
    /// The depth of the placement on its timeline.
    pub depth: u16,
    /// The sprite whose timeline holds the placement, `None` for the main timeline.
    pub sprite: Option<u16>,
}

/// A shape converted to closed fill paths and stroke paths, in its own twips.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneShape {
    /// The ID of the shape or morph shape.
    pub id: u16,
    /// The layers from bottom to top.
    pub layers: Vec<ShapeLayer>,
}

/// One path of a shape with its paint, filled or stroked.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeLayer {
    /// The outline.
    pub path: ScenePath,
    /// What it is painted with.
    pub paint: Paint,
    /// Whether it is filled or stroked.
    pub kind: LayerKind,
}

/// Whether a layer is filled or stroked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayerKind {
    /// Filled with the even-odd rule.
    Fill,
    /// Stroked.
    Stroke(StrokeStyle),
}

/// How a path is stroked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeStyle {
    /// The width in twips, 0 for a hairline.
    pub width: f32,
    /// The cap of open ends.
    pub cap: Cap,
    /// The join of corners.
    pub join: Join,
    /// The miter limit of a miter join.
    pub miter_limit: f32,
}

/// The end of an open stroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cap {
    /// Round.
    Round,
    /// Flat at the end point.
    Butt,
    /// Square beyond the end point.
    Square,
}

/// The corner of a stroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Join {
    /// Round.
    Round,
    /// Cut off.
    Bevel,
    /// Pointed up to the miter limit.
    Miter,
}

/// An outline of moves, lines and quadratic curves in twips.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScenePath {
    /// The commands in order.
    pub commands: Vec<PathCommand>,
}

/// One command of a [`ScenePath`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    /// Starts a contour.
    MoveTo(ScenePoint),
    /// A straight line.
    LineTo(ScenePoint),
    /// A quadratic Bézier curve through a control point.
    QuadTo(ScenePoint, ScenePoint),
    /// Closes the contour.
    Close,
}

/// A point in twips.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScenePoint {
    /// The horizontal coordinate.
    pub x: f32,
    /// The vertical coordinate.
    pub y: f32,
}

/// What a layer is painted with.
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    /// One colour.
    Solid(Rgba),
    /// A gradient.
    Gradient(GradientPaint),
    /// A bitmap.
    Bitmap(BitmapPaint),
}

/// A gradient in its square from −16384 to 16384 twips, mapped onto the shape.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientPaint {
    /// Linear, radial or focal.
    pub kind: GradientKind,
    /// Maps the gradient square onto the shape.
    pub transform: Affine,
    /// How the area beyond the gradient is filled.
    pub spread: Spread,
    /// The colour stops, at most [`MAX_GRADIENT_STOPS`].
    pub stops: Vec<GradientStop>,
}

/// The form of a gradient.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GradientKind {
    /// Along the x axis from −16384 to 16384.
    Linear,
    /// From the centre to the radius 16384.
    Radial,
    /// Radial with the focal point moved along the x axis, −1 to 1.
    Focal(f32),
}

/// How the area beyond a gradient is filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Spread {
    /// With the end colours.
    Pad,
    /// Mirrored.
    Reflect,
    /// Repeated.
    Repeat,
}

/// A bitmap fill, one twip per pixel before its transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BitmapPaint {
    /// The ID of the bitmap.
    pub bitmap_id: u16,
    /// Maps bitmap pixels onto the shape.
    pub transform: Affine,
    /// Whether it repeats beyond its edges; otherwise its edge pixels continue.
    pub repeating: bool,
    /// Whether it is smoothed when scaled.
    pub smoothed: bool,
}

/// A shape drawn at a place in the frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneItem {
    /// The index of the shape in [`Scene::shapes`].
    pub shape: usize,
    /// Maps the shape into the stage.
    pub transform: Affine,
    /// Changes the colours of the shape.
    pub color_transform: ColorTransform,
    /// The index of the mask in [`Scene::masks`] that clips it.
    pub mask: Option<usize>,
    /// The placement it comes from.
    pub origin: Origin,
}

/// The area that clips the items of the depths a mask placement covers: the fills of its shapes,
/// within its parent mask.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneMask {
    /// The shapes by index in [`Scene::shapes`] with their transforms into the stage.
    pub shapes: Vec<(usize, Affine)>,
    /// The mask that clips this one.
    pub parent: Option<usize>,
}

/// An affine transform: x' = a·x + c·y + tx, y' = b·x + d·y + ty.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
    /// The horizontal scale.
    pub a: f32,
    /// The vertical shear.
    pub b: f32,
    /// The horizontal shear.
    pub c: f32,
    /// The vertical scale.
    pub d: f32,
    /// The horizontal translation.
    pub tx: f32,
    /// The vertical translation.
    pub ty: f32,
}

impl Affine {
    /// The transform that changes nothing.
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    /// Returns the transform that applies `inner` first and then `self`.
    #[must_use]
    pub fn then(&self, inner: &Self) -> Self {
        Self {
            a: self.a.mul_add(inner.a, self.c * inner.b),
            b: self.b.mul_add(inner.a, self.d * inner.b),
            c: self.a.mul_add(inner.c, self.c * inner.d),
            d: self.b.mul_add(inner.c, self.d * inner.d),
            tx: self.a.mul_add(inner.tx, self.c.mul_add(inner.ty, self.tx)),
            ty: self.b.mul_add(inner.tx, self.d.mul_add(inner.ty, self.ty)),
        }
    }

    /// Returns whether every factor is finite.
    #[must_use]
    pub const fn is_finite(&self) -> bool {
        self.a.is_finite()
            && self.b.is_finite()
            && self.c.is_finite()
            && self.d.is_finite()
            && self.tx.is_finite()
            && self.ty.is_finite()
    }
}

impl From<Matrix> for Affine {
    fn from(matrix: Matrix) -> Self {
        #[expect(
            clippy::cast_precision_loss,
            reason = "translations in twips fit an f32 closely enough for drawing"
        )]
        let (tx, ty) = (matrix.translate_x as f32, matrix.translate_y as f32);
        Self {
            a: matrix.scale_x,
            b: matrix.rotate_skew0,
            c: matrix.rotate_skew1,
            d: matrix.scale_y,
            tx,
            ty,
        }
    }
}
