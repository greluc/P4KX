use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use p4kx_channels::{SearchPaths, detect};
use p4kx_settings::{Settings, default_settings_file};
use thiserror::Error;

use crate::cli::{ChannelsAction, ChannelsArgs};

const SETTINGS_FILE: &str = "settings.toml";
const UNKNOWN: &str = "unknown";

/// A `channels add` or `channels remove` argument that names nothing usable.
#[derive(Debug, Error)]
pub(crate) enum InvalidPath {
    /// The path to add does not exist.
    #[error("{0} does not exist")]
    Missing(PathBuf),
    /// The path to remove was never added.
    #[error("{0} is not a manually added folder or Archive")]
    NotAdded(PathBuf),
}

/// Lists the detected Channels, or adds or removes a manual entry in the settings.
pub(crate) fn run(args: &ChannelsArgs) -> Result<()> {
    let settings_file = settings_file(args)?;
    match &args.action {
        None => list(args, &settings_file),
        Some(ChannelsAction::Add { path }) => add(path, &settings_file),
        Some(ChannelsAction::Remove { path }) => remove(path, &settings_file),
    }
}

fn settings_file(args: &ChannelsArgs) -> Result<PathBuf> {
    args.config_dir.as_ref().map_or_else(
        || default_settings_file().context("this platform has no config directory"),
        |dir| Ok(dir.join(SETTINGS_FILE)),
    )
}

fn list(args: &ChannelsArgs, settings_file: &Path) -> Result<()> {
    let settings = Settings::load(settings_file)?;
    let standard = SearchPaths::standard();
    let paths = SearchPaths {
        default_library: args.default_library.clone().or(standard.default_library),
        launcher: args.launcher_dir.clone().or(standard.launcher),
        manual_folders: settings.channels.folders,
        manual_archives: settings.channels.archives,
    };
    let mut out = BufWriter::new(io::stdout().lock());
    write_row(&mut out, ["CHANNEL", "VERSION", "SOURCE", "ARCHIVE"])?;
    for channel in detect(&paths) {
        write_row(
            &mut out,
            [
                channel.name(),
                channel.build().map_or(UNKNOWN, |build| build.version()),
                &channel.source().to_string(),
                &channel.archive().display().to_string(),
            ],
        )?;
    }
    out.flush()?;
    Ok(())
}

fn add(path: &Path, settings_file: &Path) -> Result<()> {
    if !path.exists() {
        return Err(InvalidPath::Missing(path.to_path_buf()).into());
    }
    let path =
        std::path::absolute(path).with_context(|| format!("cannot resolve {}", path.display()))?;
    let mut settings = Settings::load(settings_file)?;
    let (kind, added) = if path.is_file() {
        ("Archive", settings.channels.add_archive(path.clone()))
    } else {
        ("folder", settings.channels.add_folder(path.clone()))
    };
    if added {
        settings.save(settings_file)?;
    }
    let verb = if added { "added" } else { "already added" };
    writeln!(io::stdout().lock(), "{verb} {kind} {}", path.display())?;
    Ok(())
}

fn remove(path: &Path, settings_file: &Path) -> Result<()> {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut settings = Settings::load(settings_file)?;
    if !settings.channels.remove(&absolute) {
        return Err(InvalidPath::NotAdded(absolute).into());
    }
    settings.save(settings_file)?;
    writeln!(io::stdout().lock(), "removed {}", absolute.display())?;
    Ok(())
}

fn write_row(
    out: &mut impl Write,
    [channel, version, source, archive]: [&str; 4],
) -> io::Result<()> {
    writeln!(out, "{channel:<14} {version:<16} {source:<9} {archive}")
}
