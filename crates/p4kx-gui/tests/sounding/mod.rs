//! Helpers of the audio preview tests: a synthetic Archive with WEMs opened in a headless
//! window that plays through a fake audio output.

use std::error::Error;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{
    AudioOutput, AudioPreview, Browser, Language, Model, Player, Preview, SoundSource, ui,
};
use p4kx_testkit::{ArchiveBuilder, TestCodec, TestEntry, TestVorbis, TestWem, test_signal};
use tempfile::TempDir;

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

/// The frames of the PCM test sound, one second at 48 kHz.
pub(crate) const TONE_FRAMES: u32 = 48_000;

/// An audio output that keeps the sources it was started with, failing them with `failure`.
#[derive(Debug, Clone, Default)]
pub(crate) struct FakeOutput {
    sources: Arc<Mutex<Vec<SoundSource>>>,
    failure: Option<&'static str>,
}

impl FakeOutput {
    pub(crate) fn failing(reason: &'static str) -> Self {
        Self {
            failure: Some(reason),
            ..Self::default()
        }
    }

    pub(crate) fn started(&self) -> usize {
        self.sources
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// Pulls `count` samples from the latest source; `None` once it ended.
    pub(crate) fn pull(&self, count: usize) -> Option<Vec<f32>> {
        let mut sources = self.sources.lock().unwrap_or_else(PoisonError::into_inner);
        let source = sources.last_mut()?;
        let mut samples = vec![0.0; count];
        let written = source.fill(&mut samples);
        drop(sources);
        (written == count).then_some(samples)
    }
}

impl AudioOutput for FakeOutput {
    fn start(&self, source: SoundSource) -> Box<dyn Send> {
        if let Some(reason) = self.failure {
            source.reporter().report(reason);
        }
        self.sources
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(source);
        Box::new(())
    }
}

pub(crate) fn tone() -> Vec<i16> {
    test_signal(TONE_FRAMES, 2)
}

/// A Wwise Vorbis tone of two channels.
pub(crate) fn vorbis() -> TestVorbis {
    TestVorbis::new(2, 48_000, &[false; 400]).tone(10)
}

pub(crate) fn sounds() -> Vec<TestEntry> {
    let pcm = TestWem::pcm(TestCodec::WwisePcm, 2, 48_000, &tone()).bytes();
    let ima = TestWem::ima_adpcm(1, 48_000, &test_signal(640, 1)).bytes();
    let opus = TestWem::encoded(TestCodec::Tag(0x3040), 1, 48_000, 960).bytes();
    let endless = TestWem::encoded(TestCodec::Vorbis, 2, 48_000, u32::MAX).bytes();
    vec![
        TestEntry::stored("Data\\Sounds\\tone.wem", pcm),
        TestEntry::zstd("Data\\Sounds\\ima.WEM", ima),
        TestEntry::stored("Data\\Sounds\\engine.wem", vorbis().wem().bytes()),
        TestEntry::stored("Data\\Sounds\\voice.wem", opus),
        TestEntry::stored("Data\\Sounds\\endless.wem", endless),
        TestEntry::stored(
            "Data\\Sounds\\broken.wem",
            b"RIFF\x04\x00\x00\x00WAVE".to_vec(),
        ),
        TestEntry::stored("Data\\readme.txt", "plain text"),
    ]
}

pub(crate) fn opened(
    language: Language,
    output: &FakeOutput,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    sounds()
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    let mut model = Model::new(language);
    model.set_audio_output(output.clone());
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1600.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok((folder, harness))
}

pub(crate) fn wait_until(
    harness: &mut Harness<'_, Model>,
    done: impl Fn(&Model) -> bool,
) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(20);
    harness.step();
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.step();
    Ok(())
}

pub(crate) fn audio(model: &Model) -> Option<&AudioPreview> {
    model
        .browser()
        .and_then(Browser::preview)
        .and_then(Preview::audio)
}

pub(crate) fn player(model: &Model) -> Option<&Player> {
    audio(model).and_then(AudioPreview::player)
}

pub(crate) fn select(harness: &mut Harness<'_, Model>, name: &str) -> TestResult {
    harness.get_by_label(name).click();
    wait_until(harness, |model| {
        audio(model).is_some_and(|audio| !audio.is_busy())
    })
}

/// Returns the sample of the test tone at `frame` and `channel` as the output gets it.
pub(crate) fn tone_sample(frame: usize, channel: usize) -> f32 {
    tone()
        .get(frame * 2 + channel)
        .map_or(0.0, |&sample| f32::from(sample) / 32_768.0)
}
