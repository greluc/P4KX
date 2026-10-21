//! A synthetic Archive with split textures and helpers for texture Conversion tests.

use std::error::Error;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use p4kx_extraction::{
    Archive, ConflictPolicy, ExtractOptions, ExtractionReport, TextureConversion, extract,
};
use p4kx_testkit::{ArchiveBuilder, SplitDds, TestEntry, TestTextureFormat};

pub(crate) const HEADER: &str = "Data\\Textures\\rock_diff.dds";

pub(crate) fn rock() -> SplitDds {
    SplitDds::new(TestTextureFormat::LegacyDxt5, 32, 16).split(3)
}

pub(crate) fn texture_entries(dds: &SplitDds) -> Vec<TestEntry> {
    dds.entries(HEADER)
}

pub(crate) fn merged_layout(dds: &SplitDds) -> Vec<u8> {
    let header = dds.header_bytes();
    let mut merged = header.get(..dds.header_len()).unwrap_or_default().to_vec();
    for level in 0..6 {
        merged.extend_from_slice(&dds.mip(level));
    }
    merged
}

pub(crate) struct Fixture {
    _dir: tempfile::TempDir,
    pub(crate) archive: Archive,
    pub(crate) target: PathBuf,
}

impl Fixture {
    pub(crate) fn new(entries: Vec<TestEntry>) -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("Data.p4k");
        entries
            .into_iter()
            .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
            .write_to(&path)?;
        let target = dir.path().join("out");
        fs::create_dir(&target)?;
        Ok(Self {
            archive: Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?,
            target,
            _dir: dir,
        })
    }

    pub(crate) fn read(&self, path: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(fs::read(self.target.join(path))?)
    }

    pub(crate) fn write(&self, path: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let file = self.target.join(path);
        if let Some(folder) = file.parent() {
            fs::create_dir_all(folder)?;
        }
        Ok(fs::write(file, data)?)
    }
}

pub(crate) fn run(fixture: &Fixture, policy: ConflictPolicy) -> ExtractionReport {
    run_with(fixture, policy, TextureConversion::default())
}

pub(crate) fn run_with(
    fixture: &Fixture,
    policy: ConflictPolicy,
    textures: TextureConversion,
) -> ExtractionReport {
    let entries: Vec<_> = fixture.archive.entries().iter().collect();
    let options = ExtractOptions {
        policy,
        convert: true,
        textures,
        ..ExtractOptions::default()
    };
    extract(&fixture.archive, &entries, &fixture.target, &options)
}

/// Returns every file below `root` as a sorted path with `/` separators.
pub(crate) fn files(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut found = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder)? {
            let path = entry?.path();
            if path.is_dir() {
                folders.push(path);
            } else {
                let relative = path.strip_prefix(root)?;
                let parts: Vec<_> = relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect();
                found.push(parts.join("/"));
            }
        }
    }
    found.sort();
    Ok(found)
}

pub(crate) fn read_png(path: &Path) -> Result<(u32, u32, Vec<u8>), Box<dyn Error>> {
    let mut reader = png::Decoder::new(std::io::BufReader::new(File::open(path)?)).read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size().unwrap_or_default()];
    let info = reader.next_frame(&mut pixels)?;
    Ok((info.width, info.height, pixels))
}
