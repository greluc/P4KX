// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The SVG and the raster of one scene agree where it is easy to say what is drawn: at probe
//! points inside solid shapes, the topmost placed shape of the document has the colour of the
//! pixel, and points that no shape covers are transparent in the raster and covered by nothing in
//! the document. Antialiased edges, gradients, bitmaps and masks are checked by the other SVG tests,
//! not by renderer comparison.

use std::collections::HashMap;
use std::error::Error;

use crate::svg_support::{RED, XLINK, elements, options, parse, rect, stage};
use p4kx_gfx::{CanvasSize, Movie, RgbaImage};
use p4kx_testkit::{TestFill, TestPlace};
use roxmltree::{Document, Node};

type TestResult = Result<(), Box<dyn Error>>;
type Painted = ([u8; 3], f64);

const BLUE_FILL: [u8; 4] = [0, 0, 255, 255];
const SIDE: i32 = 600;
const CANVAS: u32 = 200;

fn movie() -> Vec<u8> {
    let square = rect(4, (SIDE, SIDE), TestFill::Solid(RED));
    let blue = rect(5, (SIDE, SIDE), TestFill::Solid(BLUE_FILL));
    stage()
        .tag(square.code(), &square.body())
        .tag(blue.code(), &blue.body())
        .tag(26, &TestPlace::new(1, 4).at(200, 200).body2())
        .tag(26, &TestPlace::new(2, 5).at(400, 400).body2())
        .tag(
            26,
            &TestPlace::new(3, 4)
                .at(1200, 200)
                .color([0.0, 1.0, 1.0, 0.5], [0, 255, 0, 0])
                .body2(),
        )
        .tag(1, &[])
        .bytes()
}

struct Affine {
    xx: f64,
    yx: f64,
    xy: f64,
    yy: f64,
    dx: f64,
    dy: f64,
}

impl Affine {
    fn parse(text: &str) -> Result<Self, Box<dyn Error>> {
        let inner = text
            .strip_prefix("matrix(")
            .and_then(|rest| rest.strip_suffix(')'))
            .ok_or("a matrix")?;
        let values = inner
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<Vec<f64>, _>>()?;
        let &[xx, yx, xy, yy, dx, dy] = values.as_slice() else {
            return Err("six numbers".into());
        };
        Ok(Self {
            xx,
            yx,
            xy,
            yy,
            dx,
            dy,
        })
    }

    const fn apply(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (
            x.mul_add(self.xx, y.mul_add(self.xy, self.dx)),
            x.mul_add(self.yx, y.mul_add(self.yy, self.dy)),
        )
    }
}
fn corners(d: &str) -> Result<[(f64, f64); 2], Box<dyn Error>> {
    let numbers = d
        .replace(['M', 'L', 'Q', 'Z'], " ")
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<Vec<f64>, _>>()?;
    let mut low = (f64::MAX, f64::MAX);
    let mut high = (f64::MIN, f64::MIN);
    for &[x, y] in numbers.as_chunks::<2>().0 {
        low = (low.0.min(x), low.1.min(y));
        high = (high.0.max(x), high.1.max(y));
    }
    Ok([low, high])
}
fn group_with_id<'a, 'i>(document: &'a Document<'i>, id: &str) -> Option<Node<'a, 'i>> {
    elements(document, "g")
        .into_iter()
        .find(|node| node.attribute("id") == Some(id))
}

fn topmost_colour(
    document: &Document<'_>,
    (x, y): (f64, f64),
) -> Result<Option<Painted>, Box<dyn Error>> {
    let root = elements(document, "g")
        .into_iter()
        .find(|node| node.attribute("id").is_none() && node.attribute("transform").is_some())
        .ok_or("the root group")?;
    let root_matrix = Affine::parse(root.attribute("transform").ok_or("a transform")?)?;
    let mut found = None;
    for placed in root.children().filter(|node| node.has_tag_name("use")) {
        let id = placed
            .attribute((XLINK, "href"))
            .and_then(|href| href.strip_prefix('#'))
            .ok_or("a reference")?;
        let item_matrix = Affine::parse(
            placed
                .attribute("transform")
                .unwrap_or("matrix(1 0 0 1 0 0)"),
        )?;
        let group = group_with_id(document, id).ok_or("the referenced group")?;
        let path = group
            .children()
            .find(|node| node.has_tag_name("path"))
            .ok_or("a path")?;
        let [low, high] = corners(path.attribute("d").ok_or("path data")?)?;
        let a = root_matrix.apply(item_matrix.apply(low));
        let b = root_matrix.apply(item_matrix.apply(high));
        let (min_x, max_x) = (a.0.min(b.0), a.0.max(b.0));
        let (min_y, max_y) = (a.1.min(b.1), a.1.max(b.1));
        if (min_x..=max_x).contains(&x) && (min_y..=max_y).contains(&y) {
            let fill = path.attribute("fill").ok_or("a fill")?;
            let hex = fill.strip_prefix('#').ok_or("a hex colour")?;
            let channel = |at: usize| -> Result<u8, Box<dyn Error>> {
                Ok(u8::from_str_radix(
                    hex.get(at..at + 2).ok_or("two digits")?,
                    16,
                )?)
            };
            let opacity = path
                .attribute("fill-opacity")
                .map_or(Ok(1.0), str::parse::<f64>)?;
            found = Some(([channel(0)?, channel(2)?, channel(4)?], opacity));
        }
    }
    Ok(found)
}

#[test]
fn probes_inside_solid_shapes_have_the_colour_of_the_raster() -> TestResult {
    let bytes = movie();
    let parsed = Movie::parse(bytes)?;
    let assets = parsed.assets();
    let scene = parsed.first_frame(&assets);
    let options = options(CanvasSize::Width(CANVAS));
    let bitmaps: HashMap<u16, RgbaImage> = HashMap::new();
    let raster = scene.rasterize(&bitmaps, &options)?;
    let mut out = Vec::new();
    scene.write_svg(&bitmaps, &options, None, &mut out)?;
    let text = String::from_utf8(out)?;
    let document = parse(&text);
    let width = usize::try_from(raster.image.width())?;
    let probes: [((u32, u32), &str); 6] = [
        ((15, 15), "red only"),
        ((25, 25), "red under blue"),
        ((45, 45), "blue only"),
        ((75, 25), "tinted"),
        ((90, 90), "nothing"),
        ((5, 5), "nothing"),
    ];
    for ((px, py), what) in probes {
        let (cx, cy) = (px * 2, py * 2);
        let at = (usize::try_from(cy)? * width + usize::try_from(cx)?) * 4;
        let pixel = raster.image.pixels().get(at..at + 4).ok_or("a pixel")?;
        let drawn = topmost_colour(&document, (f64::from(cx) + 0.5, f64::from(cy) + 0.5))?;
        match drawn {
            None => assert_eq!(pixel[3], 0, "{what}: the raster draws something"),
            Some((rgb, opacity)) => {
                let alpha = (opacity * 255.0).round();
                assert!(
                    (f64::from(pixel[3]) - alpha).abs() <= 2.0,
                    "{what}: alpha {} against {alpha}",
                    pixel[3]
                );
                for (channel, expected) in pixel[..3].iter().zip(rgb) {
                    assert!(
                        channel.abs_diff(expected) <= 3,
                        "{what}: {pixel:?} against {rgb:?}"
                    );
                }
            }
        }
    }
    Ok(())
}
