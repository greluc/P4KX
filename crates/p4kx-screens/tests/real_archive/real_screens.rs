// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The screens of real ships list and render without a panic, within the limits.

use std::collections::BTreeSet;
use std::error::Error;

use p4kx_assembly::{ArchiveFiles, LoadoutResolver};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gfx::{Background, CanvasSize, RasterOptions};
use p4kx_screens::ScreenComposer;
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: [&str; 4] = [
    "AEGS_Avenger_Titan",
    "RSI_Aurora_ES",
    "ANVL_Hornet_F7A_Mk2",
    "DRAK_Cutlass_Black",
];

const AVENGER_SCREENS: [&str; 3] = [
    "libs/foundry/records/ui/buildingblocks/ships/displays/mfdscreens/mc_mfdcomponents/screens/general/m_mfd_screen",
    "libs/foundry/records/ui/buildingblocks/ships/displays/physicalscreens/annunciator/h_eng_annunciator_master_left",
    "libs/foundry/records/ui/buildingblocks/components/map/mapdisplaymaster",
];

const LAYOUT_GAPS: [&str; 6] = [
    "parents form a cycle",
    "StackContainer children",
    "GridPackContainer children",
    "no target that is a widget of the canvas",
    "the template contains a clone of itself",
    "is not finite",
];

const NO_RENDERER: &str = "no renderer for this element type";

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the summary of the run is for the reader of --nocapture"
)]
fn the_layout_gaps_of_the_avenger_screens_are_closed() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut screens = ScreenComposer::new(&forge);
    for canvas in AVENGER_SCREENS {
        let composed = screens.compose(canvas, Some("aegs"))?;
        let report = composed.report();
        composed.rasterize(&RasterOptions {
            size: CanvasSize::Width(640),
            background: Background::Transparent,
        })?;
        assert!(!report.truncated, "{canvas}: within the limits");
        for entry in &report.unsupported {
            let layout_gap = LAYOUT_GAPS
                .iter()
                .any(|reason| entry.reason.contains(reason));
            assert!(!layout_gap, "{canvas}: {} {}", entry.element, entry.reason);
            let lists_and_clones = matches!(entry.element.as_str(), "WidgetList" | "WidgetClone")
                && entry.reason.contains(NO_RENDERER);
            assert!(!lists_and_clones, "{canvas}: {}", entry.element);
        }
        println!(
            "{canvas}: {} widgets, {} shapes, {} kinds left out",
            report.widgets,
            report.shapes,
            report.unsupported.len()
        );
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the summary of the run is for the reader of --nocapture"
)]
fn the_screens_of_real_ships_list_and_render() -> TestResult {
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
    let mut rendered = BTreeSet::new();
    for ship in SHIPS {
        let Ok(record) = resolver.entities().find(ship) else {
            continue;
        };
        let tree = resolver.resolve(&record)?;
        let listing = screens.screens_of(&tree);
        assert!(!listing.screens.is_empty(), "{ship} has screens");
        for screen in &listing.screens {
            if !rendered.insert((screen.canvas.clone(), listing.brand.clone())) {
                continue;
            }
            let composed = screens.compose(&screen.canvas_guid, listing.brand.as_deref())?;
            composed.rasterize(&options)?;
            let report = composed.report();
            assert!(report.widgets > 0, "{ship} {}: widgets", screen.canvas);
            assert!(
                !report.truncated,
                "{ship} {}: within the limits",
                screen.canvas
            );
            println!(
                "{ship} {}: {} widgets, {} canvases, {} shapes, {} kinds left out",
                screen.canvas,
                report.widgets,
                report.canvases,
                report.shapes,
                report.unsupported.len()
            );
        }
    }
    assert!(!rendered.is_empty(), "at least one ship rendered");
    Ok(())
}
