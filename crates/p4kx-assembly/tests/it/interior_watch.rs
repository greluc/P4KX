// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Stopping and watching the build of an Interior, and writing it with textures.

use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_assembly::{AssemblyError, Interior, InteriorOptions, InteriorProgress, InteriorWatch};
use p4kx_extraction::{Archive, select_entries};
use p4kx_gltf::{GlbOptions, LoadedTexture, TextureLoadError, TextureLoader};
use p4kx_testkit::{INTERIOR_ROOT, interior_entries};

use crate::assembly_support::{TestResult, fixture};

fn sample(options: &InteriorOptions) -> Result<Interior, Box<dyn std::error::Error>> {
    let fixture = fixture(interior_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let found = select_entries(&archive, &[INTERIOR_ROOT])?;
    let entry = found.first().ok_or("no root container")?;
    let root = archive.open_container(entry)?;
    Ok(Interior::build(&[&archive, &root], INTERIOR_ROOT, options)?)
}

#[test]
fn building_stops_when_the_watch_asks_and_reports_the_model_files_it_reads() -> TestResult {
    let fixture = fixture(interior_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let found = select_entries(&archive, &[INTERIOR_ROOT])?;
    let entry = found.first().ok_or("no root container")?;
    let root = archive.open_container(entry)?;
    let layers = [&archive, &root];
    let options = InteriorOptions::default();

    let stopped = InteriorWatch {
        cancelled: &|| true,
        progress: &|_| {},
    };
    let result = Interior::build_watched(&layers, INTERIOR_ROOT, &options, &stopped);
    assert!(
        matches!(result, Err(AssemblyError::Cancelled)),
        "stopped before any container is read"
    );

    let seen = std::sync::Mutex::new(Vec::new());
    let watched = InteriorWatch {
        cancelled: &|| false,
        progress: &|progress| {
            if let Ok(mut seen) = seen.lock() {
                seen.push(progress);
            }
        },
    };
    let interior = Interior::build_watched(&layers, INTERIOR_ROOT, &options, &watched)?;
    let seen = seen.into_inner()?;
    assert!(
        seen.iter().any(|progress| matches!(
            progress,
            InteriorProgress::Models { total, .. } if *total > 0
        )),
        "the files to read are counted: {seen:?}"
    );
    assert_eq!(
        interior.parts().len(),
        sample(&options)?.parts().len(),
        "the same Interior as without a watch"
    );
    Ok(())
}

#[test]
fn building_stops_at_every_point_the_watch_is_asked() -> TestResult {
    let fixture = fixture(interior_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let found = select_entries(&archive, &[INTERIOR_ROOT])?;
    let entry = found.first().ok_or("no root container")?;
    let root = archive.open_container(entry)?;
    let layers = [&archive, &root];
    let options = InteriorOptions::default();
    let asked = AtomicUsize::new(0);
    let counting = InteriorWatch {
        cancelled: &|| {
            asked.fetch_add(1, Ordering::Relaxed);
            false
        },
        progress: &|_| {},
    };
    Interior::build_watched(&layers, INTERIOR_ROOT, &options, &counting)?;
    let total = asked.load(Ordering::Relaxed);
    assert!(
        total > 3,
        "the watch is asked more than once per wave: {total}"
    );

    for stop_at in 0..total {
        let calls = AtomicUsize::new(0);
        let stopping = InteriorWatch {
            cancelled: &|| calls.fetch_add(1, Ordering::Relaxed) >= stop_at,
            progress: &|_| {},
        };
        let result = Interior::build_watched(&layers, INTERIOR_ROOT, &options, &stopping);
        assert!(
            matches!(result, Err(AssemblyError::Cancelled)),
            "stopped when asked for the {stop_at}th time"
        );
    }
    Ok(())
}

#[test]
fn the_model_progress_ends_at_its_total() -> TestResult {
    let fixture = fixture(interior_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let found = select_entries(&archive, &[INTERIOR_ROOT])?;
    let entry = found.first().ok_or("no root container")?;
    let root = archive.open_container(entry)?;
    let seen = std::sync::Mutex::new(Vec::new());
    let watched = InteriorWatch {
        cancelled: &|| false,
        progress: &|progress| {
            if let (Ok(mut seen), InteriorProgress::Models { done, total }) =
                (seen.lock(), progress)
            {
                seen.push((done, total));
            }
        },
    };

    Interior::build_watched(
        &[&archive, &root],
        INTERIOR_ROOT,
        &InteriorOptions::default(),
        &watched,
    )?;

    let seen = seen.into_inner()?;
    let (done, total) = seen.last().copied().ok_or("no progress")?;
    assert!(total > 0, "files are counted");
    assert_eq!(done, total, "the last step is the whole: {seen:?}");
    Ok(())
}

struct NoTextures;

impl TextureLoader for NoTextures {
    fn load(&self, _path: &str, _mip: u32) -> Result<LoadedTexture, TextureLoadError> {
        Err(TextureLoadError::NotFound)
    }
}

#[test]
fn writing_with_textures_gives_the_plain_file_when_the_materials_are_plain() -> TestResult {
    let interior = sample(&InteriorOptions::default())?;
    let options = GlbOptions::default();
    let (mut plain, mut with) = (Vec::new(), Vec::new());

    interior.write_glb(&options, &mut plain)?;
    let summary = interior.write_glb_with(&options, &NoTextures, &mut with)?;

    assert_eq!(plain, with, "the same file");
    assert!(summary.texture_fallbacks.is_empty(), "no texture asked for");
    Ok(())
}
