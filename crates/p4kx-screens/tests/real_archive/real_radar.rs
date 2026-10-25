// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The radar screens of real ships list and render.

use std::collections::BTreeSet;
use std::error::Error;

use p4kx_assembly::{ArchiveFiles, LoadoutResolver};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gfx::{Background, CanvasSize, RasterOptions};
use p4kx_screens::ScreenComposer;
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: [&str; 14] = [
    "AEGS_Avenger_Titan",
    "AEGS_Gladius",
    "AEGS_Sabre",
    "ANVL_Hornet_F7A_Mk2",
    "ANVL_Carrack",
    "CRUS_Starfighter_Ion",
    "DRAK_Cutlass_Black",
    "DRAK_Corsair",
    "MISC_Freelancer",
    "ORIG_300i",
    "RSI_Aurora_ES",
    "RSI_Constellation_Andromeda",
    "RSI_Zeus_Mk2_ES",
    "ARGO_MPUV_Cargo",
];

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the summary of the run is for the reader of --nocapture"
)]
fn the_radar_screens_of_real_ships_render() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let mut screens = ScreenComposer::new(&forge);
    let options = RasterOptions {
        size: CanvasSize::Width(640),
        background: Background::Transparent,
    };
    let mut seen = BTreeSet::new();
    for ship in SHIPS {
        let Ok(record) = resolver.entities().find(ship) else {
            println!("{ship}: not found");
            continue;
        };
        let tree = resolver.resolve(&record)?;
        let listing = screens.screens_of(&tree);
        for screen in &listing.screens {
            println!("  {ship} [{}] {}", screen.port, screen.canvas);
            let radar = screen.port.to_lowercase().contains("radar")
                || screen.canvas.contains("radar")
                || screen.canvas.contains("/map/");
            if !radar || !seen.insert(screen.canvas.clone()) {
                continue;
            }
            let composed = screens.compose(&screen.canvas_guid, listing.brand.as_deref())?;
            composed.rasterize(&options)?;
            let report = composed.report();
            let missing: Vec<String> = report
                .unsupported
                .iter()
                .filter(|entry| entry.reason.contains("no renderer"))
                .map(|entry| format!("{} {}", entry.element, entry.count))
                .collect();
            println!(
                "{ship} [{}] {}: {} widgets; no renderer: {}",
                screen.port,
                screen.canvas,
                report.widgets,
                missing.join(", ")
            );
        }
    }
    Ok(())
}
