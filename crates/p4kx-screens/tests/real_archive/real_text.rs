// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The texts of real ship screens are drawn from the fonts of `fonts_en.gfx`.

use std::error::Error;

use p4kx_assembly::{ArchiveFiles, LoadoutResolver};
use p4kx_extraction::gfx::Localise;
use p4kx_extraction::{
    Archive, DataForgeEntry, Language, LocalizationEntry, UiMovie, decode_line_breaks,
};
use p4kx_screens::{ScreenComposer, TextSources};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: [&str; 4] = [
    "AEGS_Avenger_Titan",
    "RSI_Aurora_GS_ES",
    "ANVL_Hornet_F7A_Mk2",
    "DRAK_Cutlass_Black",
];
/// The share of widgets listed as without a renderer that fails the test, without image sources;
/// 0.09 on LIVE, with room for a changed Archive.
const MAX_NO_RENDERER_SHARE: f64 = 0.15;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    clippy::cast_precision_loss,
    reason = "the summary of the run is for the reader of --nocapture"
)]
fn the_texts_of_real_ship_screens_are_drawn() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let probe = ScreenComposer::new(&forge);
    let movie = UiMovie::from_archive(&archive, &format!("Data/{}", probe.font_file()))?;
    let fonts = movie.movie().assets().fonts;
    assert!(
        fonts
            .iter()
            .any(|font| p4kx_extraction::gfx::FontFace::new(font).is_drawable()),
        "the font movie has outlines"
    );
    let texts = LocalizationEntry::read(&archive, Language::English)?;
    let ini = texts.parse()?;
    let lookup = |key: &str| {
        ini.resolve(key)
            .map(|text| decode_line_breaks(text).into_owned())
    };
    let mut composer = ScreenComposer::new(&forge).with_texts(TextSources {
        fonts: &fonts,
        localise: Some(&lookup as Localise<'_>),
    });
    let (mut drawn, mut left_out, mut widgets, mut no_renderer) = (0, 0, 0, 0);
    for ship in SHIPS {
        let record = resolver.entities().find(ship)?;

        let tree = resolver.resolve(&record)?;
        let listing = composer.screens_of(&tree);
        let before = (no_renderer, widgets);
        for screen in &listing.screens {
            let drawn_screen = composer.compose(&screen.canvas_guid, listing.brand.as_deref())?;
            let report = drawn_screen.report();
            drawn += report.texts_drawn;
            left_out += report.texts_left_out;
            widgets += report.widgets;
            no_renderer += report
                .unsupported
                .iter()
                .filter(|entry| entry.reason.contains("no renderer"))
                .map(|entry| entry.count)
                .sum::<usize>();
        }
        let (ship_no_renderer, ship_widgets) = (no_renderer - before.0, widgets - before.1);
        println!(
            "{ship}: {ship_no_renderer} of {ship_widgets} widgets without a renderer ({:.3})",
            ship_no_renderer as f64 / ship_widgets.max(1) as f64
        );
    }
    let share = no_renderer as f64 / widgets.max(1) as f64;
    println!(
        "{drawn} texts drawn, {left_out} left out; {no_renderer} of {widgets} widgets without a renderer ({share:.2})"
    );
    assert!(drawn > 0, "some text is drawn");
    assert!(
        share < MAX_NO_RENDERER_SHARE,
        "widgets left out as without a renderer: {share:.2}"
    );
    Ok(())
}
