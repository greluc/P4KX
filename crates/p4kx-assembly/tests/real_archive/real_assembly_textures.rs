// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Textured assemblies with tint palettes of a real Archive, written as GLB.

use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::process::Command;
use std::time::Instant;

use p4kx_assembly::{
    ArchiveModels, Assembly, AssemblyOptions, ForgePalettes, IssueKind, LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::{GlbOptions, MaterialMode};
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: [&str; 2] = ["AEGS_Gladius", "ANVL_Hornet_F7C"];

fn peak_working_set() -> Option<u64> {
    let script = format!("(Get-Process -Id {}).PeakWorkingSet64", std::process::id());
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .ok()?;
    String::from_utf8(output.stdout).ok()?.trim().parse().ok()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    clippy::too_many_lines,
    reason = "the sizes, counts and times are the measurement, printed in one run"
)]
fn documented_ships_are_written_with_textures_and_palettes() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let palettes = ForgePalettes::new(&forge);
    let dir = std::env::temp_dir().join("m9-127-real");
    std::fs::create_dir_all(&dir)?;
    for name in SHIPS {
        let mut resolver = LoadoutResolver::new(&forge);
        let record = resolver.entities().find(name)?;
        let tree = resolver.resolve(&record)?;
        let started = Instant::now();
        let assembly =
            Assembly::build_with_palettes(&tree, &models, &palettes, &AssemblyOptions::default())?;
        println!("{name}: assembled in {:?}", started.elapsed());
        let painted = assembly
            .parts()
            .iter()
            .filter(|part| part.palette().is_some())
            .count();
        let unresolved: Vec<String> = assembly
            .issues()
            .iter()
            .filter(|issue| matches!(issue.kind(), IssueKind::Palette { .. }))
            .map(|issue| format!("{}: {}", issue.path(), issue.kind()))
            .collect();
        println!(
            "{name}: {} parts, {painted} with a palette, {} palette issues",
            assembly.parts().len(),
            unresolved.len()
        );
        for line in unresolved.iter().take(5) {
            println!("  {line}");
        }
        assert!(painted > 0, "{name}: no part has a palette");
        for (mode, mip) in [
            (MaterialMode::Colours, 0),
            (MaterialMode::Textures, 3),
            (MaterialMode::Textures, 1),
            (MaterialMode::Textures, 0),
        ] {
            let options = GlbOptions::default()
                .with_materials(mode)
                .with_texture_mip(mip);
            let path = dir.join(format!("{name}-{mode:?}-{mip}.glb"));
            let started = Instant::now();
            let mut writer = BufWriter::new(File::create(&path)?);
            let summary = assembly.write_glb_with(&options, &models.textures(), &mut writer)?;
            drop(writer);
            let elapsed = started.elapsed();
            println!(
                "{name} {mode:?} mip {mip}: {} bytes, {} nodes, {} meshes, {} materials ({} textured, {} shared), {} images ({} pixels, {} shared references), {} fallbacks, {} shared meshes in {elapsed:?}, peak working set {:?}",
                summary.bytes,
                summary.nodes,
                summary.meshes,
                summary.materials,
                summary.textured_materials,
                summary.shared_materials,
                summary.textures,
                summary.texture_pixels,
                summary.shared_textures,
                summary.texture_fallbacks.len(),
                summary.shared_meshes,
                peak_working_set()
            );
            for fallback in summary.texture_fallbacks.iter().take(5) {
                println!(
                    "  {} {} {}: {}",
                    fallback.material, fallback.role, fallback.file, fallback.reason
                );
            }
            if mode == MaterialMode::Textures {
                assert!(summary.textures > 0, "{name}: no image embedded");
            }
            check_glb(&std::fs::read(&path)?)?;
        }
    }
    Ok(())
}
