use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use p4kx_extraction::{Archive, AudioError, Bank, BankExtraction, EmbeddedWem, WemFormat};
use serde::Serialize;

use crate::cli::{BnkAction, BnkArgs, BnkFormat, BnkSource};
use crate::{exit, json};

const UNKNOWN: &str = "unknown";

#[derive(Serialize)]
struct ListJson<'a> {
    soundbank: &'a str,
    version: u32,
    bank_id: u32,
    wems: Vec<WemJson>,
}

#[derive(Serialize)]
struct WemJson {
    id: u32,
    size: u64,
    codec: Option<&'static str>,
    codec_name: Option<String>,
    sample_count: Option<u64>,
    duration_seconds: Option<f64>,
    prefetch: bool,
    error: Option<String>,
    #[serde(skip)]
    plugin_media: bool,
}

#[derive(Serialize)]
struct ExtractJson<'a> {
    soundbank: &'a str,
    target: String,
    format: &'static str,
    extracted: Vec<WrittenJson>,
    failed: Vec<FailedJson>,
}

#[derive(Serialize)]
struct WrittenJson {
    id: u32,
    file: String,
    size: u64,
}

#[derive(Serialize)]
struct FailedJson {
    id: u32,
    error: String,
}

/// Runs `p4kx bnk list` or `p4kx bnk extract`.
pub(crate) fn run(args: &BnkArgs, as_json: bool) -> Result<ExitCode> {
    match &args.action {
        BnkAction::List(source) => {
            list(&open(source)?, as_json)?;
            Ok(ExitCode::from(exit::SUCCESS))
        }
        BnkAction::Extract {
            source,
            out,
            format,
        } => extract(&open(source)?, out, *format, as_json),
    }
}

fn open(source: &BnkSource) -> Result<Bank> {
    Ok(match &source.entry {
        Some(entry) => Bank::from_archive(&Archive::open(&source.path)?, entry)?,
        None => Bank::from_file(&source.path)?,
    })
}

fn describe(wem: &EmbeddedWem<'_>) -> WemJson {
    let mut json = WemJson {
        id: wem.id(),
        size: wem.bytes().len() as u64,
        codec: None,
        codec_name: None,
        sample_count: None,
        duration_seconds: None,
        prefetch: wem.is_prefetch(),
        error: None,
        plugin_media: false,
    };
    if json.prefetch {
        return json;
    }
    match wem.parse() {
        Ok(parsed) => {
            json.codec = Some(parsed.codec().id());
            json.codec_name = Some(parsed.codec().to_string());
            json.sample_count = parsed.sample_count();
            json.duration_seconds = parsed.duration().map(|duration| duration.as_secs_f64());
        }
        Err(error) => {
            json.plugin_media = matches!(error, AudioError::PluginMedia);
            json.error = Some(error.to_string());
        }
    }
    json
}

fn list(bank: &Bank, as_json: bool) -> Result<()> {
    let soundbank = bank.soundbank();
    let wems: Vec<WemJson> = soundbank.wems().map(|wem| describe(&wem)).collect();
    if as_json {
        json::print(&ListJson {
            soundbank: bank.name(),
            version: soundbank.header().version,
            bank_id: soundbank.header().id,
            wems,
        })?;
        return Ok(());
    }
    let mut out = BufWriter::new(io::stdout().lock());
    writeln!(out, "Soundbank:   {}", bank.name())?;
    writeln!(out, "Version:     {}", soundbank.header().version)?;
    writeln!(out, "Bank ID:     {}", soundbank.header().id)?;
    writeln!(out, "WEMs:        {}", wems.len())?;
    if !wems.is_empty() {
        writeln!(out)?;
        writeln!(out, "{:>10}  {:<16}  {:>10}", "ID", "Codec", "Duration")?;
    }
    for wem in &wems {
        let codec = match &wem.codec_name {
            Some(name) => name,
            None if wem.prefetch => "prefetch",
            None if wem.plugin_media => "plug-in media",
            None => "invalid",
        };
        let duration = wem
            .duration_seconds
            .map_or_else(|| UNKNOWN.to_owned(), |seconds| format!("{seconds:.3} s"));
        write!(out, "{:>10}  {codec:<16}  {duration:>10}", wem.id)?;
        if let Some(error) = wem.error.as_ref().filter(|_| !wem.plugin_media) {
            write!(out, "  {error}")?;
        }
        writeln!(out)?;
    }
    out.flush()?;
    Ok(())
}

fn extract(bank: &Bank, out: &Path, format: BnkFormat, as_json: bool) -> Result<ExitCode> {
    let format = match format {
        BnkFormat::Wem => WemFormat::Wem,
        BnkFormat::Wav => WemFormat::Wav,
    };
    let extraction = bank.extract_wems(out, format)?;
    let total = extraction.written.len() + extraction.failures.len();
    if as_json {
        json::print(&extract_json(bank, out, format, &extraction))?;
    } else {
        let mut stderr = io::stderr().lock();
        for failure in &extraction.failures {
            writeln!(stderr, "error: {}", chain(&failure.error))?;
        }
        writeln!(
            io::stdout().lock(),
            "extracted {} of {total} WEMs from {} into {}",
            extraction.written.len(),
            bank.name(),
            out.display()
        )?;
    }
    Ok(ExitCode::from(if extraction.failures.is_empty() {
        exit::SUCCESS
    } else {
        exit::PARTIAL_FAILURE
    }))
}

fn extract_json<'a>(
    bank: &'a Bank,
    out: &Path,
    format: WemFormat,
    extraction: &BankExtraction,
) -> ExtractJson<'a> {
    ExtractJson {
        soundbank: bank.name(),
        target: json::path(out),
        format: format.extension(),
        extracted: extraction
            .written
            .iter()
            .map(|written| WrittenJson {
                id: written.id,
                file: json::path(&written.path),
                size: written.size,
            })
            .collect(),
        failed: extraction
            .failures
            .iter()
            .map(|failure| FailedJson {
                id: failure.id,
                error: chain(&failure.error),
            })
            .collect(),
    }
}

fn chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut text = String::new();
    let mut current = Some(error);
    while let Some(error) = current {
        if !text.is_empty() {
            text.push_str(": ");
        }
        text.push_str(&error.to_string());
        current = error.source();
    }
    text
}
