// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `ScreenComposer::screens_of` over the synthetic sample ship.

use p4kx_assembly::LoadoutResolver;
use p4kx_dataforge::DataForge;
use p4kx_screens::{MovieUse, ScreenComposer};
use p4kx_testkit::screens_sample;

use crate::support::TestResult;

#[test]
fn the_screens_of_a_ship_are_listed_with_their_assets() -> TestResult {
    let bytes = screens_sample().build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge);
    let ship = resolver.entities().find("TEST_ScreenShip")?;
    let tree = resolver.resolve(&ship)?;
    let listing = ScreenComposer::new(&forge).screens_of(&tree);
    assert_eq!(listing.entity, "TEST_ScreenShip", "the root Entity");
    assert_eq!(
        listing.brand.as_deref(),
        Some("test"),
        "the brand of the class prefix"
    );
    let ports: Vec<&str> = listing.screens.iter().map(|s| s.port.as_str()).collect();
    assert_eq!(
        ports,
        ["Left_MFD", "Radar"],
        "one screen per port, in Loadout order"
    );
    let mfd = listing.screens.first().ok_or("no MFD screen")?;
    assert_eq!(mfd.entity, "TEST_Screen_MFD", "the Entity that holds it");
    assert_eq!(
        mfd.canvas, "libs/foundry/records/ui/buildingblocks/test/test_mfd",
        "the record path of the canvas"
    );
    assert_eq!(
        mfd.field, "Components[0].layers[0].views[0].component.canvas",
        "the field that refers to it"
    );
    assert_eq!((mfd.width, mfd.height), (200.0, 100.0), "the canvas size");
    assert_eq!(
        mfd.assets.canvases,
        [
            "libs/foundry/records/ui/buildingblocks/test/test_mfd",
            "libs/foundry/records/ui/buildingblocks/test/test_panel"
        ],
        "the screen's own canvas first, then the canvases it nests"
    );
    assert_eq!(
        mfd.assets.textures,
        ["UI/Textures/Test/picture.dds"],
        "a .tif texture is listed as the .dds the Archive holds"
    );
    assert_eq!(
        mfd.assets.vectors,
        ["UI/Textures/Vector/Test/glyph.svg"],
        "vector graphics of nested canvases are listed"
    );
    assert_eq!(
        mfd.assets.localisation_keys,
        ["test_label"],
        "keys lose the @"
    );
    Ok(())
}

#[test]
fn element_types_are_counted_and_marked_as_drawn_or_not() -> TestResult {
    let bytes = screens_sample().build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge);
    let ship = resolver.entities().find("TEST_ScreenShip")?;
    let tree = resolver.resolve(&ship)?;
    let listing = ScreenComposer::new(&forge).screens_of(&tree);
    let mfd = listing.screens.first().ok_or("no MFD screen")?;
    let find = |element: &str| mfd.elements.iter().find(|entry| entry.element == element);
    let card = find("WidgetCard").ok_or("no card")?;
    assert_eq!((card.count, card.rendered), (1, true), "a card is drawn");
    let text = find("WidgetText").ok_or("no text")?;
    assert_eq!(
        (text.count, text.rendered),
        (1, true),
        "a text has a renderer, which needs the fonts"
    );
    let image = find("WidgetImage").ok_or("no image")?;
    assert!(
        image.rendered,
        "an image has a renderer, which needs image sources"
    );
    let shapes = find("WidgetCustomShape").ok_or("no custom shape of the nested canvas")?;
    assert_eq!(shapes.count, 1, "widgets of nested canvases count too");
    Ok(())
}

#[test]
fn ui_movies_of_the_entities_are_listed() -> TestResult {
    let bytes = screens_sample().build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge);
    let ship = resolver.entities().find("TEST_ScreenShip")?;
    let tree = resolver.resolve(&ship)?;
    let listing = ScreenComposer::new(&forge).screens_of(&tree);
    assert_eq!(
        listing.movies,
        [MovieUse {
            port: "Left_MFD".to_owned(),
            entity: "TEST_Screen_MFD".to_owned(),
            movie: "Data/UI/test.gfx".to_owned(),
        }],
        "the movie path of the UI owner component, with its port"
    );
    Ok(())
}

#[test]
fn the_brand_is_the_manufacturer_prefix_when_the_archive_has_its_style() -> TestResult {
    let bytes = screens_sample().build();
    let forge = DataForge::open(&bytes)?;
    let composer = ScreenComposer::new(&forge);
    assert_eq!(
        composer.brand_of("TEST_ScreenShip").as_deref(),
        Some("test"),
        "the style s_test_hud is in the Archive"
    );
    assert_eq!(composer.brand_of("NONE_Ship"), None, "no style s_none_hud");
    Ok(())
}
