// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The textures and vector graphics of real ship screens are drawn.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use p4kx_assembly::{ArchiveFiles, LoadoutResolver};
use p4kx_extraction::gfx::Localise;
use p4kx_extraction::{
    Archive, ArchiveImages, DataForgeEntry, Language, LocalizationEntry, UiMovie,
    decode_line_breaks,
};
use p4kx_gfx::{Background, CanvasSize, RasterOptions};
use p4kx_screens::{ImageSources, MAX_SVG_BYTES, Report, ScreenComposer, TextSources};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: [&str; 4] = [
    "AEGS_Avenger_Titan",
    "RSI_Aurora_ES",
    "ANVL_Hornet_F7A_Mk2",
    "DRAK_Cutlass_Black",
];
const MAX_NO_RENDERER_SHARE: f64 = 0.15;

#[derive(Default)]
struct Tally {
    drawn: usize,
    left_out: usize,
    widgets: usize,
    no_renderer: usize,
    kinds: BTreeMap<String, usize>,
    unreadable: BTreeSet<String>,
}

impl Tally {
    fn add(&mut self, report: &Report) {
        self.drawn += report.images_drawn;
        self.left_out += report.images_left_out;
        self.widgets += report.widgets;
        for entry in &report.unsupported {
            if entry.reason.contains("no renderer") {
                self.no_renderer += entry.count;
                *self.kinds.entry(entry.element.clone()).or_default() += entry.count;
            }
            if entry.reason.contains("unreadable") {
                self.unreadable.extend(entry.examples.iter().cloned());
            }
        }
    }

    #[expect(clippy::cast_precision_loss, reason = "the counts are far below 2^52")]
    fn share(&self) -> f64 {
        self.no_renderer as f64 / self.widgets.max(1) as f64
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    clippy::too_many_lines,
    reason = "the summary of the run is for the reader of --nocapture, and one test sets up the whole Archive"
)]
fn the_images_of_real_ship_screens_are_drawn() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let font_file = ScreenComposer::new(&forge).font_file();
    let movie = UiMovie::from_archive(&archive, &format!("Data/{font_file}"))?;
    let fonts = movie.movie().assets().fonts;
    let texts = LocalizationEntry::read(&archive, Language::English)?;
    let ini = texts.parse()?;
    let lookup = |key: &str| {
        ini.resolve(key)
            .map(|text| decode_line_breaks(text).into_owned())
    };
    let images = ArchiveImages::new(&archive);
    let texture = |path: &str, side: u32| images.texture(path, side).map_err(|e| e.to_string());
    let vector = |path: &str| {
        images
            .vector(path, MAX_SVG_BYTES as u64)
            .map_err(|e| e.to_string())
    };
    let mut screens = ScreenComposer::new(&forge)
        .with_texts(TextSources {
            fonts: &fonts,
            localise: Some(&lookup as Localise<'_>),
        })
        .with_images(ImageSources {
            texture: &texture,
            vector: &vector,
        });
    let options = RasterOptions {
        size: CanvasSize::Width(960),
        background: Background::Transparent,
    };
    let mut tally = Tally::default();
    for ship in SHIPS {
        let Ok(record) = resolver.entities().find(ship) else {
            continue;
        };
        let tree = resolver.resolve(&record)?;
        let listing = screens.screens_of(&tree);
        for screen in &listing.screens {
            let drawn = screens.compose(&screen.canvas_guid, listing.brand.as_deref())?;
            drawn.rasterize(&options)?;
            tally.add(drawn.report());
        }
    }
    println!("without a renderer: {:?}", tally.kinds);
    for path in &tally.unreadable {
        println!("unreadable: {path}");
    }
    println!(
        "{} images drawn, {} left out; {} of {} widgets without a renderer ({:.3})",
        tally.drawn,
        tally.left_out,
        tally.no_renderer,
        tally.widgets,
        tally.share()
    );
    assert!(tally.drawn > 0, "some image is drawn");
    assert!(
        tally.share() < MAX_NO_RENDERER_SHARE,
        "widgets left out as without a renderer: {:.3}",
        tally.share()
    );
    Ok(())
}
