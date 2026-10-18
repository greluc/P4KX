//! `p4kx`, the command-line interface of P4K-Extractor.

mod cli;
mod exit;
mod extract;
mod list;

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::List(args) => list::run(&args).map(|()| ExitCode::from(exit::SUCCESS)),
        Command::Extract(args) => extract::run(&args),
    };
    match result {
        Ok(code) => code,
        Err(error) if is_broken_pipe(&error) => ExitCode::from(exit::SUCCESS),
        Err(error) => {
            let _ignored = writeln!(io::stderr().lock(), "error: {error:#}");
            exit::for_error(&error)
        }
    }
}

fn is_broken_pipe(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<io::Error>()
        .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
}
