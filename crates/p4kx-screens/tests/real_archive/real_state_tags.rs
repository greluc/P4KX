// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The MFD of the Avenger Titan, powered on and at rest, with state tags evaluated.

use std::error::Error;

use p4kx_extraction::gfx::Localise;
use p4kx_extraction::{
    Archive, ArchiveImages, DataForgeEntry, Language, LocalizationEntry, UiMovie,
    decode_line_breaks,
};
use p4kx_screens::{ImageSources, MAX_SVG_BYTES, ScreenComposer, TextSources, Variable};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const MFD: &str = "libs/foundry/records/ui/buildingblocks/ships/displays/mfdscreens/mc_mfdcomponents/screens/general/m_mfd_screen";
const BRAND: &str = "aegs";

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_powered_mfd_draws_shapes_and_texts_and_lists_the_textures_the_archive_lacks() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
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
    let texture = |path: &str, side: u32, pixels: u64| {
        images
            .texture(path, side, pixels)
            .map_err(|error| error.to_string())
    };
    let vector = |path: &str| {
        images
            .vector(path, MAX_SVG_BYTES as u64)
            .map_err(|error| error.to_string())
    };
    let compose = |settings: &[Variable]| {
        ScreenComposer::new(&forge)
            .with_texts(TextSources {
                fonts: &fonts,
                localise: Some(&lookup as Localise<'_>),
            })
            .with_images(ImageSources {
                texture: &texture,
                vector: &vector,
            })
            .with_variables(settings)
            .compose(MFD, Some(BRAND))
    };
    let at_rest = compose(&[])?;
    let powered = compose(&[Variable::new("powerstate", "1")?])?;
    assert_eq!(at_rest.report().shapes, 0, "a cold MFD draws nothing");
    assert!(
        powered.report().shapes > 0,
        "{} shapes",
        powered.report().shapes
    );
    assert!(
        powered.report().texts_drawn > 0,
        "{} texts",
        powered.report().texts_drawn
    );
    assert!(
        powered
            .report()
            .missing_textures
            .iter()
            .all(|path| path.contains("MFDs")),
        "{:?}",
        powered.report().missing_textures
    );
    Ok(())
}
