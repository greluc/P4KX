//! `p4kx`, the command-line interface of P4K-Extractor.

mod audio;
mod bnk;
mod catalog;
mod channels;
mod chunks;
mod cli;
mod cli_audio;
mod cli_bnk;
mod cli_catalog;
mod cli_chunks;
mod cli_dcb;
mod cli_entity;
mod cli_ui;
mod compare;
mod cryxml;
mod dataforges;
mod dcb;
mod dcb_export;
mod dcb_query;
mod diff;
mod entity;
mod entity_json;
mod exit;
mod extract;
mod info;
mod json;
mod list;
mod progress;
mod query_file;
mod query_rows;
mod query_table;
mod texture;
mod texture_image;
mod ui;
mod ui_json;

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.json;
    let success = |()| ExitCode::from(exit::SUCCESS);
    let result = match cli.command {
        Command::List(args) => list::run(&args, json).map(success),
        Command::Search(args) => list::search(&args, json).map(success),
        Command::Extract(args) => extract::run(&args, json),
        Command::Channels(args) => channels::run(&args, json).map(success),
        Command::Info(args) => info::run(&args, json).map(success),
        Command::Dcb(args) => dcb::run(&args, json),
        Command::Catalog(args) => catalog::run(&args, json).map(success),
        Command::Entity(args) => entity::run(&args, json).map(success),
        Command::Compare(args) => {
            return compare::run(&args, json).unwrap_or_else(|error| {
                report(&error);
                exit::for_compare_error(&error)
            });
        }
        Command::CryXml(args) => cryxml::run(&args, json).map(success),
        Command::Texture(args) => texture::run(&args, json).map(success),
        Command::Audio(args) => audio::run(&args, json).map(success),
        Command::Bnk(args) => bnk::run(&args, json),
        Command::Ui(args) => ui::run(&args, json),
        Command::Chunks(args) => chunks::run(&args, json).map(success),
    };
    match result {
        Ok(code) => code,
        Err(error) if is_broken_pipe(&error) => ExitCode::from(exit::SUCCESS),
        Err(error) => {
            report(&error);
            exit::for_error(&error)
        }
    }
}

fn report(error: &anyhow::Error) {
    let _ignored = writeln!(io::stderr().lock(), "error: {error:#}");
}

fn is_broken_pipe(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<io::Error>()
        .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
}
