// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The names of the Archive resolve the hashes of the shipped CHF files, and the labelled JSON
//! still gives the original bytes.

use std::collections::{BTreeSet, HashSet};
use std::error::Error;
use std::io::Read;
use std::path::Path;

use p4kx_assembly::character::{Chf, ItemPort, NameTable};
use p4kx_assembly::{ArchiveModels, ForgeItems, LoadoutResolver, archive_names};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const PORTS_ONLY_THE_ARCHIVE_NAMES: [u32; 2] = [0x6d6d_e693, 0x6958_d171];

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the shares are the measurement")]
fn the_archive_names_resolve_the_hashes_of_every_shipped_character() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge);
    let mut items = ForgeItems::new(&mut resolver);
    let builtin = NameTable::builtin();
    let (mut files, mut builtin_named, mut named, mut unresolved) = (0_usize, 0_usize, 0_usize, 0);
    let mut distinct = BTreeSet::new();
    let mut seen = HashSet::new();
    for file in archive.entries() {
        let is_chf = Path::new(file.path())
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("chf"));
        if !is_chf {
            continue;
        }
        let mut bytes = Vec::new();
        archive.open_entry(file)?.read_to_end(&mut bytes)?;
        let chf = Chf::read(&bytes)?;
        files += 1;
        if let Some(root) = &chf.character.item_ports {
            collect(root, &mut seen);
        }
        builtin_named += chf.to_json_named(&builtin)?.1.named;
        let table = archive_names(&chf.character, &mut items, &models);
        let (json, summary) = chf.to_json_named(&table)?;
        assert_eq!(
            Chf::from_json(json.as_bytes())?.write()?,
            bytes,
            "{}",
            file.path()
        );
        named += summary.named;
        unresolved += summary.unresolved;
        distinct.extend(summary.unresolved_hashes);
    }
    println!(
        "{files} CHF files: {builtin_named} hashes named by the built-in table, {named} with the \
         Archive, {unresolved} unresolved ({} distinct: {distinct:08x?})",
        distinct.len()
    );
    assert!(files > 0 && named >= builtin_named && named > 0);
    for hash in PORTS_ONLY_THE_ARCHIVE_NAMES {
        assert!(seen.contains(&hash), "{hash:#010x} occurs in a shipped CHF");
        assert!(!distinct.contains(&hash), "{hash:#010x} resolves");
    }
    Ok(())
}

fn collect(port: &ItemPort, into: &mut HashSet<u32>) {
    into.insert(port.port);
    for child in &port.children {
        collect(child, into);
    }
}
