//! Assemblies of documented ships of a real Archive, exported as GLB.

use std::error::Error;
use std::time::Instant;

use p4kx_assembly::{ArchiveModels, Assembly, AssemblyOptions, LoadoutResolver, Placement};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: [&str; 5] = [
    "AEGS_Gladius",
    "AEGS_Avenger_Titan",
    "ANVL_Hornet_F7C",
    "RSI_Constellation_Andromeda",
    "TMBL_Cyclone",
];

const LOD: u32 = 2;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn documented_ships_assemble_into_valid_glbs_with_few_skipped_parts() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    for name in SHIPS {
        let started = Instant::now();
        let mut resolver = LoadoutResolver::new(&forge);
        let record = resolver.entities().find(name)?;
        let tree = resolver.resolve(&record)?;
        let assembly = Assembly::build(&tree, &models, &AssemblyOptions::default().with_lod(LOD))?;
        let mut bytes = Vec::new();
        let options = GlbOptions::default().with_lod(LOD);
        let summary = assembly.write_glb(&options, &mut bytes)?;
        check_glb(&bytes)?;
        let skipped = assembly
            .issues()
            .iter()
            .filter(|issue| issue.kind().skips())
            .count();
        let placed = assembly
            .parts()
            .iter()
            .filter(|part| matches!(part.placement(), Placement::Helper(_) | Placement::PortNode))
            .count();
        report(name, &assembly, summary.bytes, skipped, placed, started);
        assert!(
            assembly.parts().len() > 20,
            "{name}: {} parts",
            assembly.parts().len()
        );
        assert!(
            assembly.modelled_parts() * 3 >= assembly.parts().len(),
            "{name}: {} of {} parts have models",
            assembly.modelled_parts(),
            assembly.parts().len()
        );
        assert!(
            skipped * 10 <= assembly.parts().len() + skipped,
            "{name}: {skipped} skipped: {:?}",
            assembly
                .issues()
                .iter()
                .filter(|issue| issue.kind().skips())
                .take(5)
                .collect::<Vec<_>>()
        );
        assert!(
            summary.parts >= assembly.modelled_parts().saturating_sub(1),
            "{name}"
        );
    }
    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "the share of placed and skipped parts and the time are the measurement"
)]
fn report(
    name: &str,
    assembly: &Assembly,
    bytes: u64,
    skipped: usize,
    placed: usize,
    started: Instant,
) {
    println!(
        "{name}: {} parts, {} with models, {placed} placed by a helper or port node, {skipped} skipped, {bytes} bytes in {:?}",
        assembly.parts().len(),
        assembly.modelled_parts(),
        started.elapsed()
    );
}
