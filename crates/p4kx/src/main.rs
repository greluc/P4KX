//! `p4kx`, the command-line interface of P4K-Extractor.

mod audio;
mod bnk;
mod channels;
mod cli;
mod cli_bnk;
mod compare;
mod cryxml;
mod dataforges;
mod dcb;
mod dcb_export;
mod dcb_query;
mod diff;
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
