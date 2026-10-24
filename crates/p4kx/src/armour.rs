// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, BufWriter, Write};

use anyhow::Result;
use p4kx_assembly::BodyVariant;
use serde::Serialize;

use crate::cli_armour::{ArmourAction, ArmourArgs};
use crate::{armour_glb, json};

#[derive(Serialize)]
struct BodyJson {
    code: &'static str,
    tag: &'static str,
    skeleton: &'static str,
    skin: &'static str,
}

#[derive(Serialize)]
struct BodiesJson {
    bodies: Vec<BodyJson>,
}

/// Runs a `p4kx armour` subcommand.
pub(crate) fn run(args: &ArmourArgs, as_json: bool) -> Result<()> {
    match &args.action {
        ArmourAction::Bodies => bodies(as_json),
        ArmourAction::Glb(args) => armour_glb::run(args, as_json),
    }
}

fn bodies(as_json: bool) -> Result<()> {
    if as_json {
        json::print(&BodiesJson {
            bodies: BodyVariant::ALL
                .into_iter()
                .map(|variant| BodyJson {
                    code: variant.code(),
                    tag: variant.tag(),
                    skeleton: variant.skeleton(),
                    skin: variant.skin(),
                })
                .collect(),
        })?;
        return Ok(());
    }
    let printed = (|| {
        let mut out = BufWriter::new(io::stdout().lock());
        for variant in BodyVariant::ALL {
            writeln!(
                out,
                "{}  tag {}  skeleton {}  skin {}",
                variant.code(),
                variant.tag(),
                variant.skeleton(),
                variant.skin()
            )?;
        }
        out.flush()
    })();
    match printed {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error.into()),
        _ => Ok(()),
    }
}
