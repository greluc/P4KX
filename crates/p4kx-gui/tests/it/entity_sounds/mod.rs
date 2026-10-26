// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the Sounds tab tests: a synthetic Archive with an Entity, ATL definitions and a
//! soundbank, opened in a headless window that plays through a fake audio output.

use std::error::Error;

use egui_kittest::Harness;
use egui_kittest::wgpu::WgpuTestRenderer;
use p4kx_gui::{Language, Model, WorkTab, ui};
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestAtl, TestBank, TestCodec, TestEntity, TestEntry,
    TestHircObject, TestWem, audio_schema, entity_schema, test_signal, wwise_short_id,
};
use tempfile::TempDir;

use crate::sounding::FakeOutput;
use crate::waiting::wait_for_model;

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

const PLAY: u16 = 0x0403;
const EMBEDDED: u8 = 0;
const STREAMED: u8 = 2;

fn pcm() -> Vec<u8> {
    TestWem::pcm(TestCodec::WwisePcm, 1, 48_000, &test_signal(4_800, 1)).bytes()
}

fn bank() -> Vec<u8> {
    let mut bank = TestBank::new(wwise_short_id("Test"))
        .wem(100, pcm())
        .wem(200, pcm())
        .object(TestHircObject::event(
            wwise_short_id("Play_Multi"),
            &[1, 2, 3],
        ))
        .object(TestHircObject::event(wwise_short_id("Play_Single"), &[4]));
    for (action, target) in [(1, 10), (2, 11), (3, 12), (4, 20)] {
        bank = bank.object(TestHircObject::action(action, PLAY, target));
    }
    bank.object(TestHircObject::sound(10, 100, EMBEDDED, 0))
        .object(TestHircObject::sound(11, 103, STREAMED, 0))
        .object(TestHircObject::sound(12, 104, STREAMED, 0))
        .object(TestHircObject::sound(20, 200, EMBEDDED, 0))
        .bytes()
}

fn voice_bank() -> Vec<u8> {
    TestBank::new(wwise_short_id("Play_DX_Hello"))
        .object(TestHircObject::event(wwise_short_id("Play_DX_Hello"), &[1]))
        .object(TestHircObject::action(1, PLAY, 10))
        .object(TestHircObject::sound(10, 8000, STREAMED, 0))
        .bytes()
}

/// Writes the Archive and opens it in a window of `size`; the Sounds tab is shown.
pub(crate) fn opened(
    language: Language,
    size: [f32; 2],
    output: &FakeOutput,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    opened_with(language, size, output, None)
}

/// Like [`opened`], rendering with `renderer` when there is one.
pub(crate) fn opened_with(
    language: Language,
    size: [f32; 2],
    output: &FakeOutput,
    renderer: Option<WgpuTestRenderer>,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let ship = TestEntity::new(
        0x0079_c5d5_1678_4f8c_85ba_18ca_8f64_2af6,
        "TEST_Ship",
        "libs/foundry/records/entities/spaceships/test_ship.xml",
    )
    .audio_trigger("Play_Single")
    .audio_trigger("Play_Multi")
    .audio_trigger("Play_Undefined");
    let database = audio_schema(entity_schema(DataForgeBuilder::new())).record(ship.record());
    let definitions = TestAtl::new("SC_Audio")
        .trigger("Play_Multi", &["Play_Multi"])
        .trigger("Play_Single", &["Play_Single"])
        .localised_trigger("Play_Voice", "Play_DX_Hello");
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    let mut archive = ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database.build()))
        .entry(TestEntry::zstd(
            "Data\\Libs\\GameAudio\\SC_Audio.xml",
            definitions.to_cryxml(),
        ))
        .entry(TestEntry::stored("Data\\Sounds\\wwise\\Test.bnk", bank()))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\Media\\103.wem",
            pcm(),
        ));
    for folder in ["English(US)", "German"] {
        archive = archive
            .entry(TestEntry::stored(
                format!("Data\\Sounds\\wwise\\{folder}\\Voice.bnk"),
                voice_bank(),
            ))
            .entry(TestEntry::stored(
                format!("Data\\Sounds\\wwise\\Media\\{folder}\\8000.wem"),
                pcm(),
            ));
    }
    archive.write_to(&path)?;
    let mut model = Model::new(language);
    model.set_audio_output(output.clone());
    let builder = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size(size);
    let builder = match renderer {
        Some(renderer) => builder.renderer(renderer),
        None => builder,
    };
    let mut harness = builder.build_ui_state(
        |ui, model: &mut Model| {
            ui::show(ui, model);
        },
        model,
    );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    harness.state_mut().set_work_tab(WorkTab::Sounds);
    wait_until(&mut harness, |model| {
        matches!(model.sounds().status(), p4kx_gui::SoundsStatus::Ready(_))
    })?;
    Ok((dir, harness))
}

pub(crate) fn wait_until(
    harness: &mut Harness<'_, Model>,
    done: impl Fn(&Model) -> bool,
) -> TestResult {
    wait_for_model(harness, "the window to settle", done)?;
    harness.step();
    Ok(())
}
