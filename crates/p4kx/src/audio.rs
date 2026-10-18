use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use p4kx_extraction::{Archive, Sound};
use serde::Serialize;

use crate::cli::{AudioAction, AudioArgs, AudioSource};
use crate::json;

const UNKNOWN: &str = "unknown";

#[derive(Serialize)]
struct InfoJson<'a> {
    sound: &'a str,
    codec: &'static str,
    codec_name: String,
    format_tag: u16,
    channels: u16,
    sample_rate: u32,
    sample_count: Option<u64>,
    duration_seconds: Option<f64>,
}

#[derive(Serialize)]
struct WavJson<'a> {
    sound: &'a str,
    output: String,
    channels: u16,
    sample_rate: u32,
    sample_count: u64,
    size: u64,
}

/// Runs `p4kx audio info` or `p4kx audio wav`.
pub(crate) fn run(args: &AudioArgs, as_json: bool) -> Result<()> {
    match &args.action {
        AudioAction::Info(source) => info(&open(source)?, as_json),
        AudioAction::Wav { source, out } => wav(&open(source)?, out, as_json),
    }
}

fn open(source: &AudioSource) -> Result<Sound> {
    Ok(match &source.entry {
        Some(entry) => Sound::from_archive(&Archive::open(&source.path)?, entry)?,
        None => Sound::from_file(&source.path)?,
    })
}

fn info(sound: &Sound, as_json: bool) -> Result<()> {
    let wem = sound.wem();
    let duration = wem.duration();
    let info = InfoJson {
        sound: sound.name(),
        codec: wem.codec().id(),
        codec_name: wem.codec().to_string(),
        format_tag: wem.format_tag(),
        channels: wem.channels(),
        sample_rate: wem.sample_rate(),
        sample_count: wem.sample_count(),
        duration_seconds: duration.map(|duration| duration.as_secs_f64()),
    };
    if as_json {
        json::print(&info)?;
        return Ok(());
    }
    let lines = [
        ("Sound", info.sound.to_owned()),
        ("Codec", info.codec_name),
        ("Channels", info.channels.to_string()),
        ("Sample rate", format!("{} Hz", info.sample_rate)),
        (
            "Duration",
            duration.map_or_else(|| UNKNOWN.to_owned(), format_duration),
        ),
    ];
    let mut out = io::stdout().lock();
    for (label, value) in lines {
        writeln!(out, "{:<12} {value}", format!("{label}:"))?;
    }
    Ok(())
}

fn format_duration(duration: Duration) -> String {
    let millis = duration.as_millis();
    format!("{}.{:03} s", millis / 1000, millis % 1000)
}

fn wav(sound: &Sound, out: &Path, as_json: bool) -> Result<()> {
    let report = sound.write_wav_to_file(out)?;
    if as_json {
        json::print(&WavJson {
            sound: sound.name(),
            output: json::path(out),
            channels: report.channels,
            sample_rate: report.sample_rate,
            sample_count: report.sample_count,
            size: report.size,
        })?;
    } else {
        writeln!(
            io::stdout().lock(),
            "wrote {} as WAV to {}, {} bytes",
            sound.name(),
            out.display(),
            report.size
        )?;
    }
    Ok(())
}
