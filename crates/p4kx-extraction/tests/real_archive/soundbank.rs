// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Every real soundbank parses, its embedded WEMs read like loose ones and its `HIRC` objects decode.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Read, Write};

use p4kx_extraction::{
    Archive, AudioError, Bank, EmbeddedWem, Entry, HircKind, HircValue, MediaClass, MediaStorage,
    SoundSource, Soundbank, WemFormat,
};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

use crate::TestResult;

const EXTRACTED_BANKS: usize = 3;

#[derive(Default)]
struct Tally {
    banks: usize,
    wems: usize,
    classes: BTreeMap<&'static str, usize>,
    sources_without_file: BTreeMap<&'static str, usize>,
    prefetch: usize,
    codecs: BTreeMap<String, usize>,
    kinds: BTreeMap<String, usize>,
    versions: BTreeMap<u32, usize>,
    media: Vec<(u32, u64)>,
    sources: Vec<(MediaStorage, u32, u32)>,
    failures: Vec<String>,
}

impl Tally {
    fn merge(mut self, other: Self) -> Self {
        self.banks += other.banks;
        self.wems += other.wems;
        for (class, count) in other.classes {
            *self.classes.entry(class).or_default() += count;
        }
        for (kind, count) in other.sources_without_file {
            *self.sources_without_file.entry(kind).or_default() += count;
        }
        self.prefetch += other.prefetch;
        self.media.extend(other.media);
        self.sources.extend(other.sources);
        for (codec, count) in other.codecs {
            *self.codecs.entry(codec).or_default() += count;
        }
        for (kind, count) in other.kinds {
            *self.kinds.entry(kind).or_default() += count;
        }
        for (version, count) in other.versions {
            *self.versions.entry(version).or_default() += count;
        }
        self.failures.extend(other.failures);
        self
    }
}

#[test]
#[expect(clippy::too_many_lines, reason = "one scenario over a real Archive")]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_real_soundbank_parses_with_its_wems_and_hirc_objects() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let banks: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            let path = entry.path().as_bytes();
            path.len() > 4 && path[path.len() - 4..].eq_ignore_ascii_case(b".bnk")
        })
        .collect();
    assert!(!banks.is_empty(), "the Archive holds soundbanks");

    let tally = banks
        .par_iter()
        .map(|entry| tally_bank(&archive, entry))
        .reduce(Tally::default, Tally::merge);
    let found = sources_found(&archive, &tally);
    writeln!(
        std::io::stderr(),
        "{} banks of versions {:?}, {} embedded media items by class {:?} ({} prefetched starts), codecs {:?}, \
         Sound sources without a file {:?}, HIRC kinds {:?}, Sound sources found: {:?}",
        tally.banks,
        tally.versions,
        tally.wems,
        tally.classes,
        tally.prefetch,
        tally.codecs,
        tally.sources_without_file,
        tally.kinds,
        found
    )?;
    assert!(
        tally.failures.is_empty(),
        "{} failures: {:#?}",
        tally.failures.len(),
        tally.failures.iter().take(20).collect::<Vec<_>>()
    );
    assert_eq!(tally.banks, banks.len(), "every bank parsed");
    assert_decoded_kinds(&tally);
    for (storage, (present, total)) in &found {
        assert!(
            *total > 0 && *present * 10 >= *total * 9,
            "{storage}: only {present} of {total} Sound sources exist, so their layout is misread"
        );
    }

    let target = tempfile::tempdir()?;
    for entry in banks
        .iter()
        .filter(|entry| entry.uncompressed_size() > 0)
        .take(EXTRACTED_BANKS)
    {
        let bank = Bank::from_archive(&archive, entry.path())?;
        let extraction = bank.extract_wems(target.path(), WemFormat::Wem)?;
        assert!(extraction.failures.is_empty(), "{}", entry.path());
        assert_eq!(
            extraction.written.len(),
            bank.soundbank().wems().len(),
            "{}: every WEM written",
            entry.path()
        );
    }
    Ok(())
}

fn assert_decoded_kinds(tally: &Tally) {
    for kind in ["Sound", "Event"] {
        assert!(
            tally.kinds.get(kind).is_some_and(|&count| count > 0),
            "{kind} objects"
        );
    }
}

fn tally_bank(archive: &Archive, entry: &Entry) -> Tally {
    let mut tally = Tally::default();
    let mut bytes = Vec::new();
    let read = archive
        .open_entry(entry)
        .map_err(|error| error.to_string())
        .and_then(|mut reader| {
            reader
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())
        });
    if let Err(error) = read {
        tally.failures.push(format!("{}: {error}", entry.path()));
        return tally;
    }
    let bank = match Soundbank::parse(bytes.as_slice()) {
        Ok(bank) => bank,
        Err(error) => {
            tally.failures.push(format!("{}: {error}", entry.path()));
            return tally;
        }
    };
    tally.banks = 1;
    tally.versions.insert(bank.header().version, 1);
    for wem in bank.wems() {
        tally_wem(&mut tally, entry, &wem);
    }
    tally
        .media
        .extend(bank.wems().map(|wem| (wem.id(), wem.bytes().len() as u64)));
    for object in bank.objects() {
        *tally.kinds.entry(kind_name(object.kind)).or_default() += 1;
        if let Some(error) = object.error() {
            tally
                .failures
                .push(format!("{} object {}: {error}", entry.path(), object.id));
        }
        if let HircValue::Sound(source) = &object.value {
            count_source_plugins(&mut tally, source);
            tally
                .sources
                .push((source.storage, source.source_id, source.in_memory_size));
        }
    }
    tally
}

fn tally_wem(tally: &mut Tally, entry: &Entry, wem: &EmbeddedWem<'_>) {
    tally.wems += 1;
    let class = wem.class();
    *tally.classes.entry(class.name()).or_default() += 1;
    if wem.is_prefetch() {
        tally.prefetch += 1;
        return;
    }
    if class.is_non_sound() {
        return;
    }
    match wem.parse() {
        Ok(parsed) => *tally.codecs.entry(parsed.codec().to_string()).or_default() += 1,
        Err(AudioError::NotWem) if class == MediaClass::Unknown => {}
        Err(error) => {
            tally
                .failures
                .push(format!("{} WEM {}: {error}", entry.path(), wem.id()));
        }
    }
}

fn count_source_plugins(tally: &mut Tally, source: &SoundSource) {
    for (flag, name) in [
        (source.is_source_plugin(), "plugin"),
        (source.is_external_source(), "external"),
        (source.is_midi(), "midi"),
    ] {
        if flag {
            *tally.sources_without_file.entry(name).or_default() += 1;
        }
    }
}

fn kind_name(kind: HircKind) -> String {
    format!("{kind:?}")
}

fn sources_found(archive: &Archive, tally: &Tally) -> BTreeMap<String, (usize, usize)> {
    let in_banks: HashMap<u32, u64> = tally.media.iter().copied().collect();
    let loose: HashSet<u32> = archive
        .entries()
        .iter()
        .filter_map(|entry| {
            let name = entry.path().rsplit('\\').next()?;
            name.strip_suffix(".wem")?.parse().ok()
        })
        .collect();
    let mut found: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut sizes = (0, 0);
    for (storage, id, in_memory_size) in &tally.sources {
        if let Some(len) = in_banks.get(id) {
            sizes.0 += usize::from(*len == u64::from(*in_memory_size));
            sizes.1 += 1;
        }
        let present = match storage {
            MediaStorage::Embedded => in_banks.contains_key(id) || loose.contains(id),
            _ => loose.contains(id),
        };
        let counts = found.entry(format!("{storage:?}")).or_default();
        counts.0 += usize::from(present);
        counts.1 += 1;
    }
    found.insert("in-memory size equals the bank's media".to_owned(), sizes);
    found
}
