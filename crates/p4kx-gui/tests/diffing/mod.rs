//! Helpers of the line diff tests: a pair of synthetic Archives with changed text, `CryXml` and
//! binary Entries.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, CryXmlByteOrder, CryXmlSignature, TestElement, TestEntry, write_build_manifest,
};
use tempfile::TempDir;

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

/// Two Archives in the Channels LIVE and PTU.
pub(crate) struct Pair {
    pub(crate) old: PathBuf,
    pub(crate) new: PathBuf,
}

/// The changed text Entry.
pub(crate) const TEXT: &str = "Data\\a.cfg";
/// The changed text Entry with two changes far apart.
pub(crate) const FAR: &str = "Data\\far.txt";
/// The changed `CryXml` Entry.
pub(crate) const CRYXML: &str = "Data\\vehicle.xml";
/// The changed binary Entry.
pub(crate) const BINARY: &str = "Data\\tex.dds";

fn write(
    dir: &Path,
    channel: &str,
    version: &str,
    builder: &ArchiveBuilder,
) -> Result<PathBuf, Box<dyn Error>> {
    let folder = dir.join(channel);
    fs::create_dir_all(&folder)?;
    write_build_manifest(&folder, "sc-alpha-4.3", version, "10203", "2026")?;
    let path = folder.join("Data.p4k");
    builder.write_to(&path)?;
    Ok(path)
}

fn vehicle(damage: &str) -> TestElement {
    TestElement::new("Vehicle")
        .attribute("name", "AEGS_Gladius")
        .child(TestElement::new("Part").attribute("damageMax", damage))
        .child(TestElement::new("Description").content("Light fighter"))
}

/// Returns the lines `line 1` up to `line {last}`, with `changed` written in words.
pub(crate) fn numbered(last: u32, changed: &[u32]) -> String {
    (1..=last).fold(String::new(), |mut text, line| {
        if changed.contains(&line) {
            text.push_str("changed ");
        }
        text.push_str("line ");
        text.push_str(&line.to_string());
        text.push('\n');
        text
    })
}

/// Writes LIVE and PTU where a text, a `CryXml` and a binary Entry changed into a folder that
/// lives as long as the returned directory.
pub(crate) fn pair() -> Result<(TempDir, Pair), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let old = ArchiveBuilder::new()
        .entry(TestEntry::stored(TEXT, "a\nb\nc\nd\n"))
        .entry(TestEntry::stored(FAR, numbered(40, &[])))
        .entry(TestEntry::stored(
            CRYXML,
            vehicle("2500").to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little),
        ))
        .entry(TestEntry::stored(BINARY, vec![0, 1, 2, 3]));
    let new = ArchiveBuilder::new()
        .entry(TestEntry::deflated(TEXT, "a\nB\nc\nd\ne\n"))
        .entry(TestEntry::stored(FAR, numbered(40, &[2, 35])))
        .entry(TestEntry::stored(
            CRYXML,
            vehicle("3000").to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little),
        ))
        .entry(TestEntry::stored(BINARY, vec![0, 1, 2, 4]));
    let old = write(dir.path(), "LIVE", "4.3.1.10203", &old)?;
    let new = write(dir.path(), "PTU", "4.3.2.10400", &new)?;
    Ok((dir, Pair { old, new }))
}
