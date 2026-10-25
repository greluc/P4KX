// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Image tab for array and volume textures: the layers are named and tiled on one sheet.

use crate::imaging;

use egui_kittest::kittest::Queryable;
use imaging::{TestResult, opened_with, select, shown_level};
use p4kx_gui::Language;
use p4kx_testkit::{SplitDds, TestTextureFormat};

const ARRAY: &str = "Data\\Textures\\stack_diff.dds";
const VOLUME: &str = "Data\\Textures\\noise_vol.dds";
const RGBA8: TestTextureFormat = TestTextureFormat::Dx10Pixel {
    dxgi_format: 28,
    pixel_bytes: 4,
};

#[test]
fn an_array_is_shown_as_a_sheet_and_the_layers_are_counted() -> TestResult {
    let entries = SplitDds::new(RGBA8, 16, 8).array(5).split(2).entries(ARRAY);
    let (_folder, mut harness) = opened_with(Language::BritishEnglish, entries)?;

    select(&mut harness, "stack_diff.dds")?;

    assert!(
        harness
            .query_by_label("Format R8G8B8A8_UNORM Size 16 × 8 Mips 5 Cubemap faces – Layers 5")
            .is_some(),
        "the facts name the layers"
    );
    assert_eq!(shown_level(&harness), Some(0), "the largest mip is shown");
    assert!(
        harness.query_by_label("Mip 0: 48 × 16 pixels").is_some(),
        "five elements in three columns and two rows"
    );
    Ok(())
}

#[test]
fn a_volume_is_shown_as_a_sheet_of_its_slices_in_german() -> TestResult {
    let entries = SplitDds::new(TestTextureFormat::LegacyBgra8, 8, 8)
        .volume(4)
        .split(1)
        .entries(VOLUME);
    let (_folder, mut harness) = opened_with(Language::German, entries)?;

    select(&mut harness, "noise_vol.dds")?;

    assert!(
        harness
            .query_by_label("Format B8G8R8A8_UNORM Größe 8 × 8 Mips 4 Cubemap-Seiten – Ebenen 4")
            .is_some(),
        "the facts name the layers"
    );
    assert!(
        harness.query_by_label("Mip 0: 16 × 16 Pixel").is_some(),
        "four slices in two columns and two rows"
    );
    Ok(())
}
