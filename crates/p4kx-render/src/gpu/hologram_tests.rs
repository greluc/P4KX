// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;
use std::sync::Arc;

use super::{
    Adapters, AssemblyDraw, Colours, Frame, Hologram, Look, Offscreen, Picture, Shading, Subject,
};
use crate::assembly::synthetic::ship;
use crate::{Camera, DrawList, HologramView};

const SIZE: [u32; 2] = [192, 128];
const TRANSPARENT: [u8; 4] = [0, 0, 0, 0];
const CHANNEL_TOLERANCE: u8 = 24;
const DIFFERING_ONE_IN: usize = 40;
const REQUIRE: &str = "P4KX_REQUIRE_SOFTWARE_ADAPTER";
const BLESS: &str = "P4KX_BLESS_REFERENCE";
const SAVE: &str = "P4KX_RENDER_DIR";

fn frame(size: [u32; 2], background: [u8; 4], look: Hologram) -> Frame {
    let scene = Arc::new(ship());
    let aspect = f64::from(size[0]) / f64::from(size[1]);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "an aspect ratio of a test size is far inside the range of an f32"
    )]
    let aspect = aspect as f32;
    let mut camera = Camera::holographic(scene.bounds(), &HologramView::default());
    camera.fit(aspect);
    let uniforms = camera.uniforms(aspect);
    let hidden = vec![false; scene.parts().len()];
    let list = Arc::new(DrawList::of(
        &scene,
        &hidden,
        &uniforms.view_proj,
        f32::from(u16::try_from(size[1]).unwrap_or(u16::MAX)),
    ));
    Frame {
        subject: Subject::Assembly(AssemblyDraw { scene, list }),
        uniforms,
        size,
        look: Look::default(),
        colours: Colours {
            background,
            ..Colours::default()
        },
        shading: Shading::Hologram(look),
    }
}

#[expect(
    clippy::print_stderr,
    reason = "tells the test run why a test checked nothing"
)]
fn software(test: &str) -> Option<Offscreen> {
    match Offscreen::new(Adapters::Software) {
        Ok(offscreen) => Some(offscreen),
        Err(error) => {
            assert!(
                std::env::var_os(REQUIRE).is_none(),
                "{test}: {REQUIRE} is set but there is no software adapter: {error}"
            );
            eprintln!("{test}: no software adapter ({error}), nothing checked");
            None
        }
    }
}

fn pixel(picture: &Picture, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * picture.width() + x) * 4) as usize;
    picture
        .rgba()
        .get(at..at + 4)
        .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
        .unwrap_or_default()
}

fn pixels(picture: &Picture) -> impl Iterator<Item = [u8; 4]> + '_ {
    picture.rgba().as_chunks::<4>().0.iter().copied()
}

fn drawn(look: Hologram) -> Option<Picture> {
    let mut offscreen = software("the hologram")?;
    Some(
        offscreen
            .draw(&frame(SIZE, TRANSPARENT, look))
            .unwrap_or_else(|error| panic!("drawing failed: {error}")),
    )
}

#[test]
fn the_hologram_is_a_tinted_shell_on_a_transparent_background() {
    let Some(picture) = drawn(Hologram::default()) else {
        return;
    };

    assert_eq!(pixel(&picture, 0, 0), TRANSPARENT, "the corner stays clear");
    let covered: Vec<[u8; 4]> = pixels(&picture).filter(|pixel| pixel[3] > 0).collect();
    assert!(
        covered.len() > 400,
        "the shell covers pixels: {}",
        covered.len()
    );
    let tinted = covered
        .iter()
        .filter(|[red, _, blue, _]| blue > red)
        .count();
    assert!(
        tinted * 10 > covered.len() * 9,
        "the shell is blue: {tinted} of {}",
        covered.len()
    );
    assert!(
        covered.iter().any(|pixel| pixel[3] < 200),
        "the body is transparent"
    );
}

#[test]
fn the_edges_glow_brighter_and_more_opaque_than_the_body() {
    let Some(picture) = drawn(Hologram::default()) else {
        return;
    };

    let covered: Vec<[u8; 4]> = pixels(&picture).filter(|pixel| pixel[3] > 0).collect();
    let alpha_of = |pixel: &[u8; 4]| u32::from(pixel[3]);
    let lowest = covered.iter().map(alpha_of).min().unwrap_or(0);
    let highest = covered.iter().map(alpha_of).max().unwrap_or(0);

    assert!(
        highest > lowest * 2,
        "the alpha rises towards the edges: {lowest} to {highest}"
    );
}

#[test]
fn a_clear_shell_with_no_glow_draws_nothing() {
    let nothing = Hologram {
        opacity: 0.0,
        glow: 0.0,
        ..Hologram::default()
    };
    let Some(picture) = drawn(nothing) else {
        return;
    };

    assert!(
        pixels(&picture).all(|pixel| pixel == TRANSPARENT),
        "no pixel is covered"
    );
}

#[test]
fn an_opaque_background_shows_through_the_body() {
    let Some(mut offscreen) = software("an_opaque_background") else {
        return;
    };
    let background = [10, 20, 30, 255];

    let Ok(picture) = offscreen.draw(&frame(SIZE, background, Hologram::default())) else {
        panic!("drawing failed");
    };
    if let Some(folder) = std::env::var_os(SAVE) {
        save(
            &picture,
            &PathBuf::from(folder).join("hologram_ship_opaque.png"),
        );
    }

    assert_eq!(
        pixel(&picture, 0, 0),
        background,
        "the corner is the background"
    );
    assert!(pixels(&picture).all(|pixel| pixel[3] == 255), "all opaque");
    assert!(
        pixels(&picture).any(|pixel| pixel != background),
        "the shell is drawn over it"
    );
}

fn reference_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/reference/hologram_ship.png")
}

fn decode(bytes: &[u8]) -> Option<Picture> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    buffer.truncate(info.buffer_size());
    Picture::from_rgba(info.width, info.height, buffer)
}

fn save(picture: &Picture, path: &std::path::Path) {
    let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::File::create(path))
        .map_err(|error| error.to_string())
        .and_then(|file| picture.write_png(file).map_err(|error| error.to_string()));
    assert!(written.is_ok(), "{}: {written:?}", path.display());
}

#[test]
fn the_ship_matches_its_reference_image_within_the_tolerance() {
    let Some(picture) = drawn(Hologram::default()) else {
        return;
    };
    if let Some(folder) = std::env::var_os(SAVE) {
        save(&picture, &PathBuf::from(folder).join("hologram_ship.png"));
    }
    if std::env::var_os(BLESS).is_some() {
        save(&picture, &reference_path());
    }

    let Some(reference) = std::fs::read(reference_path())
        .ok()
        .and_then(|bytes| decode(&bytes))
    else {
        panic!(
            "no reference image at {}; draw it with {BLESS}=1 on the software adapter",
            reference_path().display()
        );
    };

    assert_eq!(
        (reference.width(), reference.height()),
        (picture.width(), picture.height()),
        "the size of the reference"
    );
    let differing = pixels(&reference)
        .zip(pixels(&picture))
        .filter(|(wanted, found)| {
            wanted
                .iter()
                .zip(found)
                .any(|(wanted, found)| wanted.abs_diff(*found) > CHANNEL_TOLERANCE)
        })
        .count();
    let total = (SIZE[0] * SIZE[1]) as usize;
    assert!(
        differing * DIFFERING_ONE_IN <= total,
        "{differing} of {total} pixels differ by more than {CHANNEL_TOLERANCE} in a channel"
    );
}
