// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The radar screens of real ships list and render.

use std::collections::BTreeSet;
use std::error::Error;

use p4kx_assembly::{ArchiveFiles, LoadoutResolver};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gfx::{Background, CanvasSize, RasterOptions};
use p4kx_screens::{Report, ScreenComposer};
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
    "RSI_Aurora_GS_ES",
    "RSI_Constellation_Andromeda",
    "RSI_Zeus_ES",
    "ARGO_MPUV_1T",
];

const HANDLED: [&str; 11] = [
    "WidgetSeparator",
    "WidgetRuntimeImage",
    "WidgetWindow",
    "WidgetDustVolume",
    "GeometryEntityWidget",
    "VehicleEntityWidget",
    "WidgetHoloVolume",
    "WidgetLight",
    "WidgetLighting",
    "WidgetLine",
    "WidgetEditBox",
];

fn assert_handled(report: &Report, ship: &str, canvas: &str) {
    let unhandled: Vec<&str> = report
        .unsupported
        .iter()
        .filter(|entry| {
            entry.reason.contains("no renderer") && HANDLED.contains(&entry.element.as_str())
        })
        .map(|entry| entry.element.as_str())
        .collect();
    assert!(
        unhandled.is_empty(),
        "{ship} {canvas}: handled widget types listed without a renderer: {unhandled:?}"
    );
}

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
        let record = resolver.entities().find(ship)?;
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
            let raster = composed.rasterize(&options)?;
            if screen.canvas.contains("components/map/mapdisplaymaster") {
                let visible = raster
                    .image
                    .pixels()
                    .chunks(4)
                    .any(|pixel| pixel.get(3) != Some(&0));
                assert!(visible, "{ship}: the radar master canvas draws pixels");
            }
            let report = composed.report();
            let missing: Vec<String> = report
                .unsupported
                .iter()
                .filter(|entry| entry.reason.contains("no renderer"))
                .map(|entry| format!("{} {}", entry.element, entry.count))
                .collect();
            assert_handled(report, ship, &screen.canvas);
            println!(
                "{ship} [{}] {}: {} widgets; no renderer: {}",
                screen.port,
                screen.canvas,
                report.widgets,
                missing.join(", ")
            );
        }
    }
    assert!(!seen.is_empty(), "a radar canvas was rendered");
    assert!(
        seen.iter().any(|canvas| canvas
            .to_lowercase()
            .contains("components/map/mapdisplaymaster")),
        "the radar master canvas is among them: {seen:?}"
    );
    Ok(())
}
