use std::collections::HashMap;

use crate::scene::{
    Affine, BitmapPaint, Cap, GradientKind, GradientPaint, Join, LayerKind, MAX_GRADIENT_STOPS,
    Paint, PathCommand, ScenePath, ScenePoint, SceneShape, ShapeLayer, Spread, StrokeStyle,
};
use crate::scene_report::SkipReason;
use crate::shape::ShapeRecord;
use crate::style::{FillStyle, Gradient, LineStyle};

const NO_BITMAP: u16 = u16::MAX;

type Twips = (i64, i64);

/// One edge of a fill, from a point to a point, curved through a control point.
#[derive(Debug, Clone, Copy)]
struct Edge {
    from: Twips,
    control: Option<Twips>,
    to: Twips,
}

impl Edge {
    const fn reversed(self) -> Self {
        Self {
            from: self.to,
            control: self.control,
            to: self.from,
        }
    }
}

/// Converts the styles and records of a shape into layers: per style group the fills, linked
/// into closed contours, and then the strokes, both in drawing order.
pub(crate) struct ShapeBuilder<'a> {
    fill_styles: &'a [FillStyle],
    line_styles: &'a [LineStyle],
    fills: Vec<Vec<Edge>>,
    lines: Vec<ScenePath>,
    line_ends: Vec<Option<Twips>>,
    layers: Vec<ShapeLayer>,
    issues: Vec<SkipReason>,
}

impl<'a> ShapeBuilder<'a> {
    /// Converts a shape of `id` with its first style arrays and its records.
    pub(crate) fn convert(
        id: u16,
        fill_styles: &'a [FillStyle],
        line_styles: &'a [LineStyle],
        records: &'a [ShapeRecord],
    ) -> (SceneShape, Vec<SkipReason>) {
        let mut builder = Self {
            fill_styles,
            line_styles,
            fills: Vec::new(),
            lines: Vec::new(),
            line_ends: Vec::new(),
            layers: Vec::new(),
            issues: Vec::new(),
        };
        builder.reset_styles(fill_styles, line_styles);
        let mut pen: Twips = (0, 0);
        let (mut fill0, mut fill1, mut line) = (0_usize, 0_usize, 0_usize);
        for record in records {
            let ShapeRecord::StyleChange(change) = record else {
                let Some(edge) = edge(pen, record) else {
                    continue;
                };
                pen = edge.to;
                if let Some(edges) = fill0
                    .checked_sub(1)
                    .and_then(|at| builder.fills.get_mut(at))
                {
                    edges.push(edge.reversed());
                }
                if let Some(edges) = fill1
                    .checked_sub(1)
                    .and_then(|at| builder.fills.get_mut(at))
                {
                    edges.push(edge);
                }
                builder.stroke(line, edge);
                continue;
            };
            if let Some(styles) = &change.new_styles {
                builder.flush();
                builder.reset_styles(&styles.fill_styles, &styles.line_styles);
                (fill0, fill1, line) = (0, 0, 0);
            }
            if let Some(to) = change.move_to {
                pen = (i64::from(to.x), i64::from(to.y));
                builder.line_ends.iter_mut().for_each(|end| *end = None);
            }
            fill0 = change.fill_style0.map_or(fill0, index);
            fill1 = change.fill_style1.map_or(fill1, index);
            line = change.line_style.map_or(line, index);
        }
        builder.flush();
        builder.issues.dedup();
        (
            SceneShape {
                id,
                layers: builder.layers,
            },
            builder.issues,
        )
    }

    fn reset_styles(&mut self, fill_styles: &'a [FillStyle], line_styles: &'a [LineStyle]) {
        self.fill_styles = fill_styles;
        self.line_styles = line_styles;
        self.fills = vec![Vec::new(); fill_styles.len()];
        self.lines = vec![ScenePath::default(); line_styles.len()];
        self.line_ends = vec![None; line_styles.len()];
    }

    fn stroke(&mut self, line: usize, edge: Edge) {
        let Some(at) = line.checked_sub(1) else {
            return;
        };
        let (Some(path), Some(end)) = (self.lines.get_mut(at), self.line_ends.get_mut(at)) else {
            return;
        };
        if *end != Some(edge.from) {
            path.commands.push(PathCommand::MoveTo(point(edge.from)));
        }
        path.commands.push(command(&edge));
        *end = Some(edge.to);
    }

    fn flush(&mut self) {
        let fills = std::mem::take(&mut self.fills);
        for (edges, style) in fills.into_iter().zip(self.fill_styles) {
            if edges.is_empty() {
                continue;
            }
            if let Some(paint) = self.paint(style) {
                self.layers.push(ShapeLayer {
                    path: link(&edges),
                    paint,
                    kind: LayerKind::Fill,
                });
            }
        }
        let lines = std::mem::take(&mut self.lines);
        for (path, style) in lines.into_iter().zip(self.line_styles) {
            if path.commands.is_empty() {
                continue;
            }
            let paint = style
                .fill
                .as_ref()
                .map_or(Some(Paint::Solid(style.color)), |fill| self.paint(fill));
            if let Some(paint) = paint {
                self.layers.push(ShapeLayer {
                    path,
                    paint,
                    kind: LayerKind::Stroke(stroke_style(style)),
                });
            }
        }
    }

    fn paint(&mut self, style: &FillStyle) -> Option<Paint> {
        let (kind, matrix, gradient) = match style {
            FillStyle::Solid { color } => return Some(Paint::Solid(*color)),
            FillStyle::Bitmap {
                bitmap_id,
                matrix,
                repeating,
                smoothed,
            } => {
                return (*bitmap_id != NO_BITMAP).then(|| {
                    Paint::Bitmap(BitmapPaint {
                        bitmap_id: *bitmap_id,
                        transform: Affine::from(*matrix),
                        repeating: *repeating,
                        smoothed: *smoothed,
                    })
                });
            }
            FillStyle::LinearGradient { matrix, gradient } => {
                (GradientKind::Linear, matrix, gradient)
            }
            FillStyle::RadialGradient { matrix, gradient } => {
                (GradientKind::Radial, matrix, gradient)
            }
            FillStyle::FocalGradient {
                matrix,
                gradient,
                focal_point,
            } => (
                GradientKind::Focal(focal_point.clamp(-1.0, 1.0)),
                matrix,
                gradient,
            ),
        };
        self.gradient(kind, Affine::from(*matrix), gradient)
    }

    fn gradient(
        &mut self,
        kind: GradientKind,
        transform: Affine,
        gradient: &Gradient,
    ) -> Option<Paint> {
        let count = gradient.stops.len();
        if count > MAX_GRADIENT_STOPS {
            self.issues.push(SkipReason::GradientStops { count });
            return None;
        }
        Some(Paint::Gradient(GradientPaint {
            kind,
            transform,
            spread: match gradient.spread {
                1 => Spread::Reflect,
                2 => Spread::Repeat,
                _ => Spread::Pad,
            },
            stops: gradient.stops.clone(),
        }))
    }
}

/// Returns the edge a record draws from `pen`, `None` for a style change.
const fn edge(pen: Twips, record: &ShapeRecord) -> Option<Edge> {
    match *record {
        ShapeRecord::StyleChange(_) => None,
        ShapeRecord::StraightEdge { delta } => Some(Edge {
            from: pen,
            control: None,
            to: moved(pen, delta.x, delta.y),
        }),
        ShapeRecord::CurvedEdge { control, anchor } => {
            let control = moved(pen, control.x, control.y);
            Some(Edge {
                from: pen,
                control: Some(control),
                to: moved(control, anchor.x, anchor.y),
            })
        }
    }
}

fn index(style: u32) -> usize {
    usize::try_from(style).unwrap_or(usize::MAX)
}

const fn moved((x, y): Twips, dx: i32, dy: i32) -> Twips {
    (x.saturating_add(dx as i64), y.saturating_add(dy as i64))
}

const fn point((x, y): Twips) -> ScenePoint {
    #[expect(
        clippy::cast_precision_loss,
        reason = "coordinates in twips fit an f32 closely enough for drawing"
    )]
    ScenePoint {
        x: x as f32,
        y: y as f32,
    }
}

fn command(edge: &Edge) -> PathCommand {
    edge.control.map_or_else(
        || PathCommand::LineTo(point(edge.to)),
        |control| PathCommand::QuadTo(point(control), point(edge.to)),
    )
}

/// Links the edges of one fill into contours, each following edges from the end of the last.
fn link(edges: &[Edge]) -> ScenePath {
    let mut starts: HashMap<Twips, Vec<usize>> = HashMap::with_capacity(edges.len());
    for (at, edge) in edges.iter().enumerate().rev() {
        starts.entry(edge.from).or_default().push(at);
    }
    let mut used = vec![false; edges.len()];
    let mut commands = Vec::with_capacity(edges.len() + edges.len() / 2);
    for (first, edge) in edges.iter().enumerate() {
        if used.get(first).copied().unwrap_or(true) {
            continue;
        }
        commands.push(PathCommand::MoveTo(point(edge.from)));
        let mut current = first;
        loop {
            if let Some(flag) = used.get_mut(current) {
                *flag = true;
            }
            let Some(edge_now) = edges.get(current) else {
                break;
            };
            commands.push(command(edge_now));
            if edge_now.to == edge.from {
                break;
            }
            let next = starts.get_mut(&edge_now.to).and_then(|candidates| {
                while let Some(candidate) = candidates.pop() {
                    if !used.get(candidate).copied().unwrap_or(true) {
                        return Some(candidate);
                    }
                }
                None
            });
            match next {
                Some(next) => current = next,
                None => break,
            }
        }
        commands.push(PathCommand::Close);
    }
    ScenePath { commands }
}

fn stroke_style(style: &LineStyle) -> StrokeStyle {
    StrokeStyle {
        width: f32::from(style.width),
        cap: match style.start_cap {
            1 => Cap::Butt,
            2 => Cap::Square,
            _ => Cap::Round,
        },
        join: match style.join {
            1 => Join::Bevel,
            2 => Join::Miter,
            _ => Join::Round,
        },
        miter_limit: style.miter_limit.unwrap_or(3.0).max(1.0),
    }
}
