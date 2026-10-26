// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The lights and light carriers of every `.soc` file in the socpaks: the colour space of the
//! stored colours, the entities that have geometry and lights, and the `LightBox` entities.

use std::error::Error;
use std::io::Read;
use std::sync::Mutex;

use p4kx_chunks::{ChunkFile, ChunkType, Container};
use p4kx_cryxml::{Element, Xml};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Default, Debug)]
struct Census {
    entities: usize,
    props_with_lights: usize,
    classed_with_lights: usize,
    light_boxes: usize,
    channels: usize,
    as_srgb_bytes: usize,
    as_linear_bytes: usize,
}

impl Census {
    const fn add(&mut self, other: &Self) {
        self.entities += other.entities;
        self.props_with_lights += other.props_with_lights;
        self.classed_with_lights += other.classed_with_lights;
        self.light_boxes += other.light_boxes;
        self.channels += other.channels;
        self.as_srgb_bytes += other.as_srgb_bytes;
        self.as_linear_bytes += other.as_linear_bytes;
    }
}

fn census_of_soc(bytes: &[u8]) -> Census {
    let mut census = Census::default();
    let Ok(file) = ChunkFile::parse(bytes) else {
        return census;
    };
    for (index, _) in file.chunks_of(ChunkType::new(Container::CrCh, 0x0004)) {
        let Some(data) = file.chunk_data(index) else {
            continue;
        };
        if let Ok(Xml::CryXml(document)) = p4kx_cryxml::read(data) {
            census.add(&read(document.root()));
        }
    }
    census
}

fn child<'a>(element: Element<'a>, name: &str) -> Option<Element<'a>> {
    element
        .children()
        .find(|found| found.name().eq_ignore_ascii_case(name))
}

fn to_srgb(value: f32) -> f32 {
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055_f32.mul_add(value.powf(1.0 / 2.4), -0.055)
    }
}

fn is_byte(value: f32) -> bool {
    let scaled = value * 255.0;
    (scaled - scaled.round()).abs() < 0.004
}

fn has_geometry(entity: Element<'_>) -> bool {
    let mut node = child(entity, "PropertiesDataCore");
    node = node.and_then(|found| child(found, "EntityGeometryResource"));
    for _ in 0..3 {
        node = node.and_then(|found| child(found, "Geometry"));
    }
    node.and_then(|found| found.attribute("path"))
        .is_some_and(|path| !path.is_empty())
}

fn colours(element: Element<'_>, census: &mut Census) {
    if element.name() == "color" {
        for channel in ["r", "g", "b"] {
            let Some(value) = element
                .attribute(channel)
                .and_then(|text| text.parse::<f32>().ok())
                .filter(|value| *value > 0.0 && *value < 1.0)
            else {
                continue;
            };
            census.channels += 1;
            census.as_srgb_bytes += usize::from(is_byte(value));
            census.as_linear_bytes += usize::from(is_byte(to_srgb(value)));
        }
    }
    for found in element.children() {
        colours(found, census);
    }
}

fn read(root: Element<'_>) -> Census {
    let mut census = Census::default();
    for entity in root.children() {
        if !entity.name().eq_ignore_ascii_case("Entity") {
            continue;
        }
        census.entities += 1;
        let lights = child(entity, "EntityComponentLightGroup")
            .and_then(|group| child(group, "BakedInLights"))
            .inspect(|baked| colours(*baked, &mut census))
            .is_some();
        if entity.attribute("EntityClass") == Some("LightBox") {
            census.light_boxes += 1;
        }
        if lights && has_geometry(entity) {
            census.props_with_lights += 1;
        }
        if lights && entity.attribute("EntityClassGUID").is_some() && !has_geometry(entity) {
            census.classed_with_lights += 1;
        }
        if let Some(component) =
            child(entity, "PropertiesDataCore").and_then(|data| child(data, "EntityComponentLight"))
        {
            colours(component, &mut census);
        }
    }
    census
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts go into the ADR")]
fn stored_light_colours_are_linear_and_no_prop_with_geometry_carries_lights() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let total = Mutex::new(Census::default());
    let socpaks: Vec<_> = archive
        .entries()
        .iter()
        .filter(|entry| entry.path().to_ascii_lowercase().ends_with(".socpak"))
        .collect();
    socpaks.par_iter().for_each(|entry| {
        let Ok(root) = archive.open_container(entry) else {
            return;
        };
        for inner in root.entries() {
            if !inner.path().to_ascii_lowercase().ends_with(".soc") {
                continue;
            }
            let mut bytes = Vec::new();
            let Ok(mut reader) = root.open_entry(inner) else {
                continue;
            };
            if reader.read_to_end(&mut bytes).is_err() {
                continue;
            }
            let found = census_of_soc(&bytes);
            if let Ok(mut total) = total.lock() {
                total.add(&found);
            }
        }
    });
    let total = total.into_inner()?;
    println!("{total:#?}");
    assert!(total.entities > 100_000, "{total:?}");
    assert!(total.channels > 100_000, "{total:?}");
    assert!(
        total.as_linear_bytes * 100 >= total.channels * 99,
        "the stored colours are linear values of 8-bit sRGB colours: {total:?}"
    );
    assert!(
        total.as_srgb_bytes * 20 < total.channels,
        "they are rarely 8-bit values as they stand: {total:?}"
    );
    assert_eq!(
        total.props_with_lights, 0,
        "no entity with geometry of its own has baked-in lights: {total:?}"
    );
    Ok(())
}
