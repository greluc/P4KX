//! The audio preview over the WEMs of a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.

use std::collections::BTreeMap;
use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_extraction::{Archive, Codec};
use p4kx_gui::{AudioPreview, AudioStatus, Browser, EntryIndex, Preview, SoundStatus, is_wem_path};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const SAMPLES: usize = 300;
const WAIT: Duration = Duration::from_mins(1);

#[derive(Debug, Default)]
struct Tally {
    played: u32,
    unsupported: u32,
    other: u32,
}

fn audio(browser: &Browser) -> Option<&AudioPreview> {
    browser.preview().and_then(Preview::audio)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_wems_of_a_real_archive_decode_for_every_codec_with_a_decoder() -> TestResult {
    let index = EntryIndex::build(Archive::open(real_archive_path()?)?);
    let sounds: Vec<u32> = (0..u32::try_from(index.len())?)
        .filter(|&id| {
            index
                .entry(id)
                .is_some_and(|entry| is_wem_path(entry.path()))
        })
        .collect();
    let step = sounds.len().div_ceil(SAMPLES).max(1);
    let mut browser = Browser::new(index);
    let mut codecs: BTreeMap<String, (Codec, Tally)> = BTreeMap::new();

    for id in sounds.into_iter().step_by(step) {
        browser.select_entry(id, || {});
        let deadline = Instant::now() + WAIT;
        while audio(&browser).is_some_and(AudioPreview::is_busy) {
            if Instant::now() > deadline {
                return Err(format!("Entry {id} did not decode in time").into());
            }
            std::thread::sleep(Duration::from_millis(2));
            browser.poll();
        }
        let Some(AudioStatus::Opened(info, sound)) = audio(&browser).map(AudioPreview::status)
        else {
            continue;
        };
        let (_, tally) = codecs
            .entry(info.codec.to_string())
            .or_insert_with(|| (info.codec, Tally::default()));
        match sound {
            SoundStatus::Ready(_) => tally.played += 1,
            SoundStatus::Unsupported(_) => tally.unsupported += 1,
            _ => tally.other += 1,
        }
    }

    assert!(!codecs.is_empty(), "the sample holds WEMs");
    for (name, (codec, tally)) in &codecs {
        let decodable = matches!(
            codec,
            Codec::Pcm | Codec::ImaAdpcm | Codec::PtAdpcm | Codec::Vorbis
        );
        if decodable {
            assert!(
                tally.played > 0 && tally.unsupported == 0,
                "{name} plays: {tally:?}"
            );
        }
    }
    Ok(())
}
