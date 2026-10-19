//! `p4kx-mcp`: the read-only MCP server of P4K-Extractor, spoken to over stdio.

mod archives;
mod budget;
mod content;
mod content_tools;
mod definitions;
mod entries;
mod error;
mod forge_params;
mod forge_shapes;
mod locate;
mod page;
mod params;
mod query;
mod query_tools;
mod record_tools;
mod records;
mod server;
mod shapes;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use p4kx_settings::default_settings_file;
use rmcp::ServiceExt;

use crate::locate::Locations;
use crate::server::Server;

const SETTINGS_FILE: &str = "settings.toml";

/// Read-only MCP server over stdio for the Data.p4k Archives of Star Citizen.
#[derive(Debug, Parser)]
#[command(name = "p4kx-mcp", version)]
struct Args {
    /// The Archive tools use when a call names none: a Channel name such as `LIVE` or the
    /// absolute path of a `.p4k` file.
    #[arg(long)]
    archive: Option<String>,
    /// The folder holding `settings.toml` with the manual Channel entries.
    #[arg(long, env = "P4KX_CONFIG_DIR")]
    config_dir: Option<PathBuf>,
    /// The RSI Launcher's application-data folder.
    #[arg(long, env = "P4KX_LAUNCHER_DIR")]
    launcher_dir: Option<PathBuf>,
    /// The default Library folder.
    #[arg(long, env = "P4KX_DEFAULT_LIBRARY")]
    default_library: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let settings_file = args
        .config_dir
        .map(|dir| dir.join(SETTINGS_FILE))
        .or_else(default_settings_file);
    let locations = Locations::new(args.default_library, args.launcher_dir, settings_file);
    let default_archive = args
        .archive
        .as_deref()
        .map(|archive| locations.resolve(archive))
        .transpose()
        .map_err(|error| anyhow::anyhow!(error.message()))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("cannot start the runtime")?;
    runtime.block_on(async {
        let service = Server::new(locations, default_archive)
            .serve(rmcp::transport::stdio())
            .await
            .context("cannot start the MCP session")?;
        service.waiting().await.context("the MCP session failed")?;
        Ok(())
    })
}
