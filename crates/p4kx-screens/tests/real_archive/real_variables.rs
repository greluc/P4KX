// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The MFD and the radar of the Avenger Titan with game variables set.

use std::error::Error;

use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gfx::{Background, CanvasSize, RasterOptions};
use p4kx_screens::{Composed, ScreenComposer, Variable};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const MFD: &str = "libs/foundry/records/ui/buildingblocks/ships/displays/mfdscreens/mc_mfdcomponents/screens/general/m_mfd_screen";
const RADAR: &str = "libs/foundry/records/ui/buildingblocks/components/map/mapdisplaymaster";
const BRAND: &str = "aegs";

fn compose(
    composer: ScreenComposer<'_, '_>,
    canvas: &str,
    settings: &[&str],
) -> Result<Composed, Box<dyn Error>> {
    let variables = settings
        .iter()
        .map(|setting| setting.parse::<Variable>())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(composer
        .with_variables(&variables)
        .compose(canvas, Some(BRAND))?)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_mfd_of_a_powered_ship_instantiates_its_content() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let at_rest = compose(ScreenComposer::new(&forge), MFD, &[])?;
    let powered = compose(
        ScreenComposer::new(&forge),
        MFD,
        &["powerstate=1", "nonsense=1"],
    )?;
    assert_eq!(at_rest.report().widgets, 1, "a cold MFD is off");
    assert!(
        powered.report().widgets > at_rest.report().widgets,
        "the powered MFD holds its content"
    );
    let listed: Vec<(&str, bool)> = powered
        .report()
        .variables
        .iter()
        .map(|variable| (variable.name.as_str(), variable.used))
        .collect();
    assert_eq!(listed, [("powerstate", true), ("nonsense", false)]);
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_radar_changes_with_the_variables_that_choose_its_view() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let options = RasterOptions {
        size: CanvasSize::Width(512),
        background: Background::Transparent,
    };
    let at_rest = compose(ScreenComposer::new(&forge), RADAR, &[])?;
    let radar = compose(
        ScreenComposer::new(&forge),
        RADAR,
        &["isradar=1", "ismapactive=1"],
    )?;
    assert_ne!(
        at_rest.rasterize(&options)?.image.pixels(),
        radar.rasterize(&options)?.image.pixels(),
        "the variables change the picture"
    );
    assert_eq!(radar.report().unknown_variables().count(), 0);
    Ok(())
}
