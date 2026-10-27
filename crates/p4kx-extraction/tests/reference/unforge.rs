// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Tree comparison of the `CryXml` unforge converted with what P4KX reads.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

use p4kx_extraction::cryxml::{self, Xml};
use p4kx_extraction::{Archive, Entry};
use p4kx_testkit::{real_archive_key, real_archive_path, reference_unforge_path};
use rayon::prelude::*;

use super::TestResult;
use super::tally::{Compared, Tally};
use super::walk::{files_under, index_entries, relative_key};
use super::xml_tree::{XmlNode, compare};

const MAX_XML_SIZE: u64 = 64 << 20;

fn our_tree(archive: &Archive, entry: &Entry) -> Result<XmlNode, String> {
    if entry.uncompressed_size() > MAX_XML_SIZE {
        return Err("larger than the comparison reads".to_owned());
    }
    let mut bytes = Vec::new();
    let mut reader = archive
        .open_entry(entry)
        .map_err(|error| error.to_string())?;
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    match cryxml::read(&bytes).map_err(|error| error.to_string())? {
        Xml::CryXml(document) => {
            XmlNode::from_cryxml(document.root()).map_err(|_| "tree too deep".to_owned())
        }
        Xml::Text(text) => {
            let text = std::str::from_utf8(text).map_err(|error| error.to_string())?;
            let parsed = roxmltree::Document::parse(text).map_err(|error| error.to_string())?;
            XmlNode::from_roxml(parsed.root_element()).map_err(|_| "tree too deep".to_owned())
        }
    }
}

fn reference_tree(file: &Path) -> Result<XmlNode, String> {
    let text = std::fs::read_to_string(file).map_err(|error| error.to_string())?;
    let parsed = roxmltree::Document::parse(&text).map_err(|error| error.to_string())?;
    XmlNode::from_roxml(parsed.root_element()).map_err(|_| "tree too deep".to_owned())
}

fn compare_file(archive: &Archive, entry: &Entry, file: &Path) -> Compared {
    match (our_tree(archive, entry), reference_tree(file)) {
        (Ok(ours), Ok(reference)) => compare(&ours, &reference),
        (Err(error), _) => Compared::Different(format!("{}: P4KX: {error}", entry.path())),
        (_, Err(error)) => Compared::Different(format!("{}: reference: {error}", file.display())),
    }
}

/// The extensions of the sources unforge converts; it writes `<stem>.xml` next to the source.
const SOURCE_EXTENSIONS: [&str; 2] = ["chrparams", "cdf"];

/// Returns the Entry file a converted `.xml` next to its source comes from: the sibling with a
/// source extension, or the file itself when it has none.
fn source_of(converted: &Path) -> PathBuf {
    SOURCE_EXTENSIONS
        .iter()
        .map(|extension| converted.with_extension(extension))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| converted.to_path_buf())
}

fn run(archive: &Archive, root: &Path) -> Result<Tally, std::io::Error> {
    let files: Vec<PathBuf> = files_under(root)?
        .into_iter()
        .filter(|file| file.extension().is_some_and(|extension| extension == "xml"))
        .collect();
    let index = index_entries(archive.entries());
    let tally = files
        .par_iter()
        .map(|file| {
            let mut tally = Tally::default();
            let Some(key) = relative_key(root, &source_of(file)) else {
                return tally;
            };
            match index.get(&key) {
                None => tally.missing_in_archive(format!("{key}: no such Entry")),
                Some(entry) => tally.record(&key, compare_file(archive, entry, file)),
            }
            tally
        })
        .reduce(Tally::default, Tally::merge);
    Ok(tally)
}

#[test]
#[ignore = "real Archive and reference output: set P4KX_DATA_P4K and P4KX_REF_UNFORGE and pass --ignored"]
fn every_cryxml_tree_unforge_converted_is_identical() -> TestResult {
    let Some(root) = reference_unforge_path() else {
        super::report("skipped: P4KX_REF_UNFORGE is not set");
        return Ok(());
    };
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let started = Instant::now();

    let tally = run(&archive, &root)?;

    super::report(&format!("unforge run took {:?}", started.elapsed()));
    tally.finish("unforge tree comparison")?;
    Ok(())
}
