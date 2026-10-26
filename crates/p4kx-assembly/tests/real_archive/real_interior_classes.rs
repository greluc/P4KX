// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Interior of real containers built with the DataForge: props from class Records, tint
//! palettes and material libraries.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::time::{Duration, Instant};

use p4kx_assembly::{Interior, InteriorForge, InteriorOptions, InteriorPartKind, InteriorWatch};
use p4kx_extraction::{Archive, DataForgeEntry, select_entries};
use p4kx_gltf::GlbOptions;
use p4kx_model::{ContainerBounds, ObjectContainer};
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const MANTIS_BODY: &str = "Data\\ObjectContainers\\Ships\\RSI\\Mantis\\base_int_body_main.socpak";
const IDRIS_FRONT: &str =
    "Data\\ObjectContainers\\Ships\\AEGS\\Idris\\base_int_mdfront_main.socpak";
const TOLERANCE: f64 = 0.01;

fn kinds(interior: &Interior) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for issue in interior.issues() {
        *counts.entry(issue.kind().code()).or_default() += 1;
    }
    counts
}

type Built = (Interior, Interior, ContainerBounds, (Duration, Duration));

fn build_both(path: &str) -> Result<Built, Box<dyn Error>> {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let found = select_entries(&archive, &[path])?;
    let entry = found.first().ok_or("no container")?;
    let root = archive.open_container(entry)?;
    let stem = path
        .rsplit('\\')
        .next()
        .and_then(|name| name.strip_suffix(".socpak"))
        .ok_or("no container name")?;
    let description = root
        .entries()
        .iter()
        .find(|file| file.path().eq_ignore_ascii_case(&format!("{stem}.xml")))
        .ok_or("no description")?;
    let mut bytes = Vec::new();
    root.open_entry(description)?.read_to_end(&mut bytes)?;
    let bounds = ObjectContainer::parse(&bytes)?
        .bounds
        .ok_or("the container declares no bounds")?;
    let options = InteriorOptions::default();
    let started = Instant::now();
    let plain = Interior::build(&[&archive, &root], entry.path(), &options)?;
    let without = started.elapsed();
    let forge_entry = DataForgeEntry::read(&archive)?;
    let forge = forge_entry.open()?;
    let classes = InteriorForge::new(&forge);
    let started = Instant::now();
    let with = Interior::build_with(
        &[&archive, &root],
        entry.path(),
        &options,
        &InteriorWatch::none(),
        Some(&classes),
    )?;
    Ok((plain, with, bounds, (without, started.elapsed())))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts go into the PR body")]
fn the_class_props_of_the_idris_front_and_the_mantis_body_lie_inside_the_bounds() -> TestResult {
    for path in [MANTIS_BODY, IDRIS_FRONT] {
        let (plain, with, bounds, times) = build_both(path)?;
        println!(
            "{path}: build {:?} without, {:?} with the forge; props {} -> {}, parts {} -> {}, palettes {}, issues {:?} -> {:?}",
            times.0,
            times.1,
            plain.props(),
            with.props(),
            plain.parts().len(),
            with.parts().len(),
            with.parts()
                .iter()
                .filter(|part| part.palette().is_some())
                .count(),
            kinds(&plain),
            kinds(&with),
        );
        assert!(with.props() > plain.props(), "{path}");
        let strays: Vec<String> = with
            .parts()
            .iter()
            .filter(|part| part.kind() == InteriorPartKind::Prop)
            .filter(|part| !bounds.contains(part.world().translation(), TOLERANCE))
            .take(5)
            .map(|part| format!("{} at {:?}", part.name(), part.world().translation()))
            .collect();
        assert!(strays.is_empty(), "{path}: {strays:?}");
        let mut bytes = Vec::new();
        with.write_glb(&GlbOptions::default(), &mut bytes)?;
        check_glb(&bytes)?;
    }
    Ok(())
}
