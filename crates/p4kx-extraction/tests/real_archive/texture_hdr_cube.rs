//! Decoding the real Archive's BC6H cubemaps face by face: every face is finite, and the faces
//! meet at their shared edges, which holds only when the faces come in DDS order.

use std::io::Write;

use p4kx_extraction::{Archive, CubemapLayout, HdrImage, Texture};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

use crate::TestResult;

const CUBEMAP_LIMIT: usize = 24;
const MAX_FACE: u32 = 128;
const POS_X: usize = 0;
const NEG_X: usize = 1;
const POS_Y: usize = 2;
const NEG_Y: usize = 3;
const POS_Z: usize = 4;
const NEG_Z: usize = 5;

/// One side of a face: its rightmost or leftmost column, or its top or bottom row.
#[derive(Debug, Clone, Copy)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

/// Two face edges: each a face index and a side.
type EdgePair = ((usize, Side), (usize, Side));

/// The edges the horizontal cross of a DDS cubemap joins.
const SEAMS: [EdgePair; 6] = [
    ((NEG_X, Side::Right), (POS_Z, Side::Left)),
    ((POS_Z, Side::Right), (POS_X, Side::Left)),
    ((POS_X, Side::Right), (NEG_Z, Side::Left)),
    ((NEG_Z, Side::Right), (NEG_X, Side::Left)),
    ((POS_Y, Side::Bottom), (POS_Z, Side::Top)),
    ((POS_Z, Side::Bottom), (NEG_Y, Side::Top)),
];

/// The same edges paired with faces they do not touch.
const STRANGERS: [EdgePair; 6] = [
    ((NEG_X, Side::Right), (POS_X, Side::Left)),
    ((POS_Z, Side::Right), (NEG_Z, Side::Left)),
    ((POS_X, Side::Right), (NEG_X, Side::Left)),
    ((NEG_Z, Side::Right), (POS_Z, Side::Left)),
    ((POS_Y, Side::Bottom), (NEG_Y, Side::Top)),
    ((NEG_Z, Side::Bottom), (POS_Y, Side::Top)),
];

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn real_bc6h_cubemaps_decode_face_by_face_with_matching_edges() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let cubemaps: Vec<Texture<'_>> = archive
        .entries()
        .par_iter()
        .filter(|entry| {
            entry
                .path()
                .as_bytes()
                .last_chunk::<4>()
                .is_some_and(|extension| extension.eq_ignore_ascii_case(b".dds"))
        })
        .filter_map(|entry| Texture::from_archive(&archive, entry.path()).ok())
        .filter(|texture| {
            let header = texture.header();
            header.format().is_high_range() && header.faces() == 6 && header.array_size() == 1
        })
        .collect();
    let (cubemaps, malformed): (Vec<Texture<'_>>, Vec<Texture<'_>>) = cubemaps
        .into_iter()
        .partition(|texture| texture.part_count().is_ok());
    assert!(!cubemaps.is_empty(), "the Archive has BC6H cubemaps");
    let stride = (cubemaps.len() / CUBEMAP_LIMIT).max(1);

    let errors: Vec<Result<(f64, f64), String>> = cubemaps
        .par_iter()
        .step_by(stride)
        .map(seam_errors)
        .collect();

    let failures: Vec<&String> = errors
        .iter()
        .filter_map(|error| error.as_ref().err())
        .collect();
    assert!(failures.is_empty(), "cubemaps failed: {failures:#?}");
    let pairs: Vec<(f64, f64)> = errors.into_iter().filter_map(Result::ok).collect();
    let seams: f64 = pairs.iter().map(|(seam, _)| seam).sum();
    let strangers: f64 = pairs.iter().map(|(_, stranger)| stranger).sum();
    let closer = pairs
        .iter()
        .filter(|(seam, stranger)| seam < stranger)
        .count();
    writeln!(
        std::io::stderr(),
        "{} BC6H cubemaps, {} checked: seam error {seams:.3}, stranger error {strangers:.3}, \
         seams closer in {closer}; skipped {} whose header tail is not a run of whole mips: {:?}",
        cubemaps.len(),
        pairs.len(),
        malformed.len(),
        malformed.iter().map(Texture::name).collect::<Vec<_>>()
    )?;
    assert!(
        seams < strangers * 0.5,
        "faces meet at their edges: {seams} against {strangers}"
    );
    assert!(
        closer * 4 >= pairs.len() * 3,
        "in most cubemaps: {closer} of {}",
        pairs.len()
    );
    export_one_cross(&cubemaps)
}

fn seam_errors(texture: &Texture<'_>) -> Result<(f64, f64), String> {
    let header = texture.header();
    let mut level = 0;
    while level + 1 < header.mip_count() && (header.width() >> level) > MAX_FACE {
        level += 1;
    }
    let faces = texture
        .decode_surfaces_hdr(level)
        .map_err(|error| format!("{}: {error}", texture.name()))?;
    if faces.len() != 6 {
        return Err(format!("{}: {} faces", texture.name(), faces.len()));
    }
    let signed = header.format().to_string() == "BC6H_SF16";
    for (face, image) in faces.iter().enumerate() {
        if let Some(value) = image
            .pixels()
            .iter()
            .find(|value| !value.is_finite() || (!signed && **value < 0.0))
        {
            return Err(format!("{} face {face}: value {value}", texture.name()));
        }
    }
    let error = |pairs: &[EdgePair]| {
        pairs
            .iter()
            .filter_map(|&((first, first_side), (second, second_side))| {
                Some(edge_error(
                    &edge(faces.get(first)?, first_side),
                    &edge(faces.get(second)?, second_side),
                ))
            })
            .sum::<f64>()
    };
    Ok((error(&SEAMS), error(&STRANGERS)))
}

fn edge(face: &HdrImage, side: Side) -> Vec<[f32; 3]> {
    let (width, height) = (face.width() as usize, face.height() as usize);
    let (pixels, _) = face.pixels().as_chunks::<3>();
    let at = |x: usize, y: usize| pixels.get(y * width + x).copied().unwrap_or_default();
    match side {
        Side::Left => (0..height).map(|y| at(0, y)).collect(),
        Side::Right => (0..height).map(|y| at(width - 1, y)).collect(),
        Side::Top => (0..width).map(|x| at(x, 0)).collect(),
        Side::Bottom => (0..width).map(|x| at(x, height - 1)).collect(),
    }
}

fn edge_error(first: &[[f32; 3]], second: &[[f32; 3]]) -> f64 {
    let compress = |value: f32| f64::from(value.max(0.0)).ln_1p();
    let total: f64 = first
        .iter()
        .zip(second)
        .flat_map(|(a, b)| {
            a.iter()
                .zip(b)
                .map(|(a, b)| (compress(*a) - compress(*b)).abs())
        })
        .sum();
    let count = u32::try_from(first.len().max(1) * 3).unwrap_or(u32::MAX);
    total / f64::from(count)
}

fn export_one_cross(cubemaps: &[Texture<'_>]) -> TestResult {
    let Some(texture) = cubemaps.first() else {
        return Ok(());
    };
    let target = tempfile::tempdir()?;
    let out = target.path().join("cube.exr");
    let level = texture.header().mip_count().saturating_sub(1).min(3);

    let report = texture.exr_to_file_as(&out, level, CubemapLayout::Cross)?;

    assert_eq!(report.files, vec![out], "one cross");
    assert_eq!(
        (report.width, report.height),
        (
            (texture.header().width() >> level).max(1) * 4,
            (texture.header().height() >> level).max(1) * 3
        ),
        "{}: four faces wide, three high",
        texture.name()
    );
    Ok(())
}
