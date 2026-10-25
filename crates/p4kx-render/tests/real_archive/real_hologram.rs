// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The holograms of real ships are drawn on the software adapter and cover part of the picture.
//!
//! A machine without a software adapter checks nothing and says so, unless
//! `P4KX_REQUIRE_SOFTWARE_ADAPTER` is set.

use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, ForgePalettes, HologramCamera,
    LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_render::{
    Adapters, AssemblyDraw, AssemblyScene, BuildOptions, Camera, Colours, DrawList, Frame,
    Hologram, HologramView, Hooks, Look, Offscreen, Shading, Subject,
};
use p4kx_settings::ModelMaterials;
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: [&str; 4] = [
    "AEGS_Avenger_Titan",
    "RSI_Aurora_GS_ES",
    "ANVL_Hornet_F7A_Mk2",
    "DRAK_Cutlass_Black",
];
const SIZE: [u32; 2] = [640, 360];
const ALPHA_FLOOR: u8 = 8;
/// The least share of the picture a see-through hologram covers; 0.022 to 0.044 on LIVE, so half
/// of the smallest value is the floor.
const MIN_COVERAGE: f64 = 0.01;
/// The largest share it may cover; a ship that fills the picture is a camera error.
const MAX_COVERAGE: f64 = 0.5;
const REQUIRE: &str = "P4KX_REQUIRE_SOFTWARE_ADAPTER";

#[expect(
    clippy::cast_precision_loss,
    reason = "a picture side is far below the 2^24 an f32 holds exactly"
)]
fn aspect() -> f32 {
    SIZE[0] as f32 / SIZE[1] as f32
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a picture side is far below the 2^24 an f32 holds exactly"
)]
fn frame(scene: &Arc<AssemblyScene>, view: &HologramView) -> Frame {
    let mut camera = Camera::holographic(scene.bounds(), view);
    camera.fit(aspect());
    let uniforms = camera.uniforms(aspect());
    let hidden = vec![false; scene.parts().len()];
    let list = Arc::new(DrawList::of(
        scene,
        &hidden,
        &uniforms.view_proj,
        SIZE[1] as f32,
    ));
    Frame {
        subject: Subject::Assembly(AssemblyDraw {
            scene: Arc::clone(scene),
            list,
        }),
        uniforms,
        size: SIZE,
        look: Look::default(),
        colours: Colours {
            background: [0, 0, 0, 0],
            ..Colours::default()
        },
        shading: Shading::Hologram(Hologram::default()),
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    reason = "the summary of the run is for the reader of --nocapture, counts are far below 2^52, and one test sets up the whole Archive"
)]
fn the_holograms_of_real_ships_are_drawn() -> TestResult {
    let mut offscreen = match Offscreen::new(Adapters::Software) {
        Ok(offscreen) => offscreen,
        Err(error) => {
            assert!(
                std::env::var_os(REQUIRE).is_none(),
                "{REQUIRE} is set but there is no software adapter: {error}"
            );
            eprintln!("no software adapter ({error}), nothing checked");
            return Ok(());
        }
    };
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge)
        .with_files(&files)
        .stopping_at_docked_vehicles();
    let models = ArchiveModels::new(&archive);
    let palettes = ForgePalettes::new(&forge);
    let camera = HologramCamera::read(&forge).unwrap_or_default();
    let view = HologramView {
        fov_degrees: camera.fov_degrees(),
        view_angle_degrees: camera.view_angle_degrees(),
        elevation_degrees: HologramView::default().elevation_degrees,
        distance_scaler: camera.distance_scaler(),
    }
    .sanitised();
    let dir = tempfile::tempdir()?;
    for ship in SHIPS {
        let record = resolver.entities().find(ship)?;
        let started = Instant::now();
        let tree = resolver.resolve(&record)?;
        let assembly =
            Assembly::build_with_palettes(&tree, &models, &palettes, &AssemblyOptions::default())?;
        let scene = Arc::new(AssemblyScene::build(
            &assembly,
            &BuildOptions {
                materials: ModelMaterials::Colours,
                textures: None,
            },
            &Hooks {
                cancelled: &|| false,
                progress: &|_| {},
            },
        )?);
        let picture = offscreen.draw(&frame(&scene, &view))?;
        let covered = picture
            .rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|pixel| pixel[3] > ALPHA_FLOOR)
            .count();
        let coverage = covered as f64 / f64::from(SIZE[0] * SIZE[1]);
        let png = dir.path().join(format!("{ship}.png"));
        picture.write_png(std::fs::File::create(&png)?)?;
        println!(
            "{ship}: {} parts, {} drawn, {} triangles, coverage {coverage:.3}, {} ms",
            assembly.parts().len(),
            scene.drawn_parts(),
            scene.placed_triangles(),
            started.elapsed().as_millis()
        );
        assert_eq!(picture.width(), SIZE[0], "{ship}: width");
        assert_eq!(picture.height(), SIZE[1], "{ship}: height");
        assert!(scene.drawn_parts() > 0, "{ship}: parts drawn");
        assert!(
            coverage > MIN_COVERAGE,
            "{ship}: the hologram covers {coverage:.3} of the picture"
        );
        assert!(
            coverage < MAX_COVERAGE,
            "{ship}: the hologram covers {coverage:.3} of the picture"
        );
        assert!(std::fs::metadata(&png)?.len() > 0, "{ship}: PNG written");
    }
    Ok(())
}
