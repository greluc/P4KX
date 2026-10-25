// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx`, the command-line interface of P4KX.

mod armour;
mod armour_glb;
mod armour_loadout;
mod audio;
mod bnk;
mod catalog;
mod catalog_json;
mod catalog_json_shops;
mod channels;
mod character_package;
mod chf;
mod chf_glb;
mod chunks;
mod cli;
mod cli_armour;
mod cli_audio;
mod cli_bnk;
mod cli_catalog;
mod cli_channels;
mod cli_character;
mod cli_chf;
mod cli_chunks;
mod cli_conflict;
mod cli_coverage;
mod cli_cryxml;
mod cli_dcb;
mod cli_dcb_fields;
mod cli_entity;
mod cli_grep;
mod cli_hologram;
mod cli_interior;
mod cli_key;
mod cli_loc;
mod cli_model;
mod cli_ships;
mod cli_socpak;
mod cli_ui;
mod cli_verify;
mod cli_wwise;
mod compare;
mod compare_schema;
mod coverage;
mod coverage_json;
mod cryxml;
mod dataforges;
mod dcb;
mod dcb_export;
mod dcb_fields;
mod dcb_fields_out;
mod dcb_query;
mod dcb_refs;
mod dcb_refs_json;
mod dcb_refs_table;
mod dcb_tags;
mod dcb_tags_json;
mod dcb_tags_table;
mod diff;
mod entity;
mod entity_audio;
mod entity_glb;
mod entity_json;
mod entity_package;
mod entity_vehicle;
mod exit;
mod extract;
mod extract_report;
mod failure_report;
mod grep;
mod grep_json;
mod grep_table;
mod guard;
mod info;
mod interior_glb;
mod issues;
mod json;
mod key;
mod list;
mod loc;
mod model;
mod model_animations;
mod model_bones;
mod model_glb;
mod model_info;
mod model_text;
mod notice;
mod progress;
mod query_file;
mod query_rows;
mod query_table;
mod ships;
mod socpak;
mod test_build;
mod texture;
mod texture_image;
mod ui;
mod ui_extract;
mod ui_hologram;
mod ui_json;
mod ui_render;
mod ui_screen_render;
mod ui_screens;
mod verify;
mod wwise;
mod wwise_events;
mod wwise_extract;
mod wwise_json;

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.json;
    let success = |()| ExitCode::from(exit::SUCCESS);
    p4kx_extraction::protect_libraries();
    if !matches!(cli.command, Command::Key(_)) {
        guard::start_game_check();
    }
    if let Err(error) = key::init(&cli) {
        report(&error);
        return exit::for_error(&error);
    }
    let given = key::Given {
        key: cli.archive_key.clone(),
        file: cli.key_file.clone(),
    };
    let result = match cli.command {
        Command::List(args) => list::run(&args, json).map(success),
        Command::Search(args) => list::search(&args, json).map(success),
        Command::Extract(args) => extract::run(&args, json),
        Command::Verify(args) => verify::run(&args, json),
        Command::Channels(args) => channels::run(&args, json).map(success),
        Command::Info(args) => info::run(&args, json).map(success),
        Command::Dcb(args) => dcb::run(&args, json),
        Command::Grep(args) => grep::run(&args, json),
        Command::Catalog(args) => catalog::run(&args, json),
        Command::Coverage(args) => coverage::run(&args, json),
        Command::Ships(args) => ships::run(&args, json),
        Command::Entity(args) => entity::run(&args, json),
        Command::Interior(args) => match args.action {
            cli_interior::InteriorAction::Glb(args) => interior_glb::run(&args, json).map(success),
        },
        Command::Armour(args) => armour::run(&args, json).map(success),
        Command::Character(args) => character_package::run(&args, json).map(success),
        Command::Loc(args) => loc::run(&args, json),
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
        Command::Socpak(args) => socpak::run(&args, json),
        Command::Chf(args) => chf::run(&args, json).map(success),
        Command::Chunks(args) => chunks::run(&args, json).map(success),
        Command::Wwise(args) => wwise::run(&args, json),
        Command::Model(args) => model::run(&args, json).map(success),
        Command::Key(args) => key::run(&args, &given, json).map(success),
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
    let mut stderr = io::stderr().lock();
    let _ignored = writeln!(stderr, "error: {error:#}");
    if let Some(hint) = key::hint(error) {
        let _ignored = writeln!(stderr, "hint: {hint}");
    }
}

fn is_broken_pipe(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<io::Error>()
        .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
}
