//! Description, WAV conversion and Ogg rebuilding of real WEM sounds.

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

use lewton::inside_ogg::OggStreamReader;

use p4kx_extraction::{Archive, AudioError, Codec, ConversionError, Entry, Sound, Wem};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

use crate::TestResult;

const SOUND_SAMPLE: usize = 2_000;
const SERVICE_SAMPLE: usize = 5;
const MAX_SOUND_SECONDS: u64 = 3_600;
const MAX_BLOCK_SIZE: u64 = 8_192;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_sample_of_real_wems_of_every_codec_is_described_and_converted_where_supported() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let wems: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            let path = entry.path().as_bytes();
            path.len() > 4 && path[path.len() - 4..].eq_ignore_ascii_case(b".wem")
        })
        .collect();
    let stride = (wems.len() / SOUND_SAMPLE).max(1);

    let outcomes: Vec<(String, Result<String, String>)> = wems
        .par_iter()
        .step_by(stride)
        .map(|entry| (entry.path().to_owned(), convert_sound(&archive, entry)))
        .collect();
    for entry in wems.iter().step_by(stride).take(SERVICE_SAMPLE) {
        let sound = Sound::from_archive(&archive, entry.path());
        assert!(
            matches!(
                sound,
                Ok(_)
                    | Err(ConversionError::Audio {
                        source: AudioError::PluginMedia,
                        ..
                    })
            ),
            "{}: {sound:?}",
            entry.path()
        );
    }

    let mut outcomes_by_kind: HashMap<String, usize> = HashMap::new();
    let mut failures = Vec::new();
    for (path, outcome) in &outcomes {
        match outcome {
            Ok(kind) => *outcomes_by_kind.entry(kind.clone()).or_default() += 1,
            Err(reason) => failures.push(format!("{path}: {reason}")),
        }
    }
    let mut summary: Vec<_> = outcomes_by_kind.iter().collect();
    summary.sort();
    writeln!(
        std::io::stderr(),
        "{} of {} WEMs sampled; per codec and outcome: {summary:?}",
        outcomes.len(),
        wems.len()
    )?;
    assert!(
        failures.is_empty(),
        "{} of {} WEMs failed: {:#?}",
        failures.len(),
        outcomes.len(),
        failures.iter().take(20).collect::<Vec<_>>()
    );
    Ok(())
}

fn convert_sound(archive: &Archive, entry: &Entry) -> Result<String, String> {
    let mut bytes = Vec::new();
    archive
        .open_entry(entry)
        .map_err(|error| error.to_string())?
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let wem = match Wem::parse(bytes.as_slice()) {
        Ok(wem) => wem,
        Err(AudioError::PluginMedia) => return Ok("plug-in media".to_owned()),
        Err(error) => return Err(error.to_string()),
    };
    let duration = wem
        .duration()
        .ok_or_else(|| format!("{} without a duration", wem.codec()))?;
    if duration.as_secs() > MAX_SOUND_SECONDS || wem.data().is_empty() {
        return Err(format!(
            "{}: implausible {duration:?} for {} data bytes",
            wem.codec(),
            wem.data().len()
        ));
    }
    let mut wav = Vec::new();
    match wem.write_wav(&mut wav) {
        Ok(report) => {
            assert_eq!(
                report.size,
                wav.len() as u64,
                "{}: reported size",
                entry.path()
            );
            Ok(format!("{}: WAV", wem.codec()))
        }
        Err(AudioError::UnsupportedCodec(Codec::Vorbis)) => rebuild_ogg(&wem, entry),
        Err(AudioError::UnsupportedCodec(codec)) if codec != Codec::Pcm => {
            Ok(format!("{codec}: no decoder yet"))
        }
        Err(error) => Err(error.to_string()),
    }
}

fn rebuild_ogg(wem: &Wem<&[u8]>, entry: &Entry) -> Result<String, String> {
    let mut ogg = Vec::new();
    let report = wem.write_ogg(&mut ogg).map_err(|error| error.to_string())?;
    assert_eq!(
        report.size,
        ogg.len() as u64,
        "{}: reported size",
        entry.path()
    );
    let mut reader = OggStreamReader::new(Cursor::new(ogg)).map_err(|error| error.to_string())?;
    let mut frames = 0_u64;
    while let Some(packet) = reader
        .read_dec_packet()
        .map_err(|error| format!("decoding the Ogg: {error}"))?
    {
        frames += packet.first().map_or(0, Vec::len) as u64;
    }
    if wem.sample_count() != Some(report.sample_count) {
        return Err(format!(
            "the last granule {} differs from the {:?} samples of the WEM",
            report.sample_count,
            wem.sample_count()
        ));
    }
    if frames.abs_diff(report.sample_count) > MAX_BLOCK_SIZE {
        return Err(format!(
            "the Ogg decodes to {frames} frames, its last granule says {}",
            report.sample_count
        ));
    }
    Ok(format!(
        "{}: Ogg with {} codebooks",
        wem.codec(),
        report.codebooks.id()
    ))
}
