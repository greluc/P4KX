//! Survey of the chunk types in a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names
//! one. The survey table is printed (`--nocapture`) and, when `P4KX_SURVEY_OUT` names a file,
//! written there as tab-separated text.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Write as _;
use std::io::Read;

use p4kx_archive::{Archive, Entry};
use p4kx_chunks::{ChunkFile, ChunkType, Container};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const CHUNK_EXTENSIONS: [&str; 17] = [
    "cgf",
    "cga",
    "skin",
    "chr",
    "cgfm",
    "cgam",
    "skinm",
    "chrm",
    "caf",
    "dba",
    "soc",
    "anim",
    "aim",
    "cigvoxel",
    "cigvoxelheader",
    "dst",
    "img",
];
const MAX_FILE_LEN: u64 = 512 << 20;
const SNIFFED_PER_EXTENSION: usize = 12;

#[derive(Default, Clone)]
struct Stat {
    chunks: u64,
    files: u64,
    extensions: BTreeMap<String, u64>,
    example: String,
    in_container: u64,
    min_size: u64,
    max_size: u64,
}

type Key = (u8, u32, u32);

#[derive(Default)]
struct Survey {
    stats: BTreeMap<Key, Stat>,
    files: u64,
    skipped: Vec<String>,
    failed: Vec<String>,
    profiles: BTreeMap<String, u64>,
}

const BOUNDING_DATA: u32 = 0x2B7E_CF9F;
const LOD_MESH_DATA: u32 = 0x58DE_1772;
const MESH_INFO: u32 = 0x9291_4444;
const ASSET_METADATA: u32 = 0xBE5E_493E;

fn word(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(bytes).ok()?))
}

fn describe(id: u32, data: &[u8]) -> Option<String> {
    match id {
        ASSET_METADATA => Some(format!(
            "AssetMetadata size {} head {:X?}",
            data.len(),
            (0..4).map(|at| word(data, at * 4)).collect::<Vec<_>>()
        )),
        BOUNDING_DATA => {
            let (dimension, bytes) = (word(data, 16)?, word(data, 20)?);
            let grid = u64::from(dimension) * u64::from(dimension) * 8;
            Some(format!(
                "BoundingData dimension {dimension} table {bytes} dimension^2*8 {} size==40+table {}",
                grid == u64::from(bytes),
                data.len() as u64 == 40 + u64::from(bytes)
            ))
        }
        LOD_MESH_DATA => {
            let count = usize::try_from(word(data, 0x28)?).ok()?;
            let mut sum = 0_u64;
            for index in 0..count {
                sum += u64::from(word(data, 0x50 + index * 8)?);
            }
            let total = sum + 0x50 + count as u64 * 8;
            Some(format!(
                "LodMeshData blob table: blobs + table within 16 bytes of chunk size {}",
                total.abs_diff(data.len() as u64) <= 16
            ))
        }
        MESH_INFO => {
            let float = |at| word(data, at).map(f32::from_bits);
            let ordered = (0..3).all(|axis| {
                float(24 + axis * 4)
                    .zip(float(36 + axis * 4))
                    .is_some_and(|(min, max)| min <= max)
            });
            Some(format!(
                "MeshInfo flags {:?} bounding box at 24..48 ordered {ordered}",
                word(data, 0)
            ))
        }
        _ => None,
    }
}

impl Survey {
    fn profile(&mut self, file: &ChunkFile<Vec<u8>>) {
        if file.container() != Container::Ivo {
            return;
        }
        for (index, chunk) in file.chunks().iter().enumerate() {
            let Some(data) = file.chunk_data(index) else {
                continue;
            };
            if let Some(text) = describe(chunk.chunk_type().id(), data) {
                *self.profiles.entry(text).or_default() += 1;
            }
        }
    }

    fn merge(mut self, other: Self) -> Self {
        for (text, count) in other.profiles {
            *self.profiles.entry(text).or_default() += count;
        }
        for (key, stat) in other.stats {
            let mine = self.stats.entry(key).or_default();
            if mine.chunks == 0 {
                mine.min_size = stat.min_size;
                mine.example.clone_from(&stat.example);
            }
            mine.min_size = mine.min_size.min(stat.min_size);
            mine.max_size = mine.max_size.max(stat.max_size);
            mine.chunks += stat.chunks;
            mine.files += stat.files;
            mine.in_container += stat.in_container;
            for (extension, count) in stat.extensions {
                *mine.extensions.entry(extension).or_default() += count;
            }
        }
        self.files += other.files;
        self.skipped.extend(other.skipped);
        self.failed.extend(other.failed);
        self
    }

    fn record(&mut self, path: &str, location: &str, file: &ChunkFile<Vec<u8>>) {
        self.files += 1;
        self.profile(file);
        let extension = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        let in_container = u64::from(location.contains(".socpak"));
        let mut seen: Vec<Key> = Vec::new();
        for chunk in file.chunks() {
            let kind = chunk.chunk_type();
            let key: Key = (
                u8::from(kind.container() == Container::CrCh),
                kind.id(),
                chunk.version(),
            );
            let stat = self.stats.entry(key).or_default();
            if stat.chunks == 0 {
                stat.min_size = chunk.size();
                stat.example = format!("{location}{path}");
            }
            stat.min_size = stat.min_size.min(chunk.size());
            stat.max_size = stat.max_size.max(chunk.size());
            stat.chunks += 1;
            stat.in_container += in_container;
            *stat.extensions.entry(extension.clone()).or_default() += 1;
            if !seen.contains(&key) {
                seen.push(key);
                stat.files += 1;
            }
        }
    }
}

fn extension_of(entry: &Entry) -> String {
    entry
        .path()
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn scan_entry(archive: &Archive, entry: &Entry, location: &str, survey: &mut Survey) {
    if entry.uncompressed_size() > MAX_FILE_LEN {
        survey
            .skipped
            .push(format!("{location}{}: too large", entry.path()));
        return;
    }
    let mut bytes = Vec::new();
    let read = archive
        .open_entry(entry)
        .map_err(|error| format!("{error:?}"))
        .and_then(|reader| {
            reader
                .take(MAX_FILE_LEN)
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())
        });
    if let Err(error) = read {
        survey
            .failed
            .push(format!("{location}{}: read {error}", entry.path()));
        return;
    }
    if !(bytes.starts_with(b"#ivo") || bytes.starts_with(b"CrCh")) {
        return;
    }
    match ChunkFile::parse(bytes) {
        Ok(file) => survey.record(entry.path(), location, &file),
        Err(error) => survey
            .failed
            .push(format!("{location}{}: {error}", entry.path())),
    }
}

fn scan_archive(archive: &Archive, location: &str) -> Survey {
    let mut survey = Survey::default();
    for entry in archive
        .entries()
        .iter()
        .filter(|entry| CHUNK_EXTENSIONS.contains(&extension_of(entry).as_str()))
    {
        scan_entry(archive, entry, location, &mut survey);
    }
    survey
}

fn scan_container(parent: &Archive, entry: &Entry) -> Survey {
    let Ok(container) = parent.open_container(entry) else {
        return Survey {
            failed: vec![format!("{}: container", entry.path())],
            ..Survey::default()
        };
    };
    let location = format!("{}!", entry.path());
    let mut survey = scan_archive(&container, &location);
    for inner in container
        .entries()
        .iter()
        .filter(|inner| extension_of(inner) == "socpak")
    {
        survey = survey.merge(scan_container(&container, inner));
    }
    survey
}

fn sniff_extensions(archive: &Archive) -> BTreeMap<String, (usize, usize)> {
    let mut by_extension: BTreeMap<String, Vec<&Entry>> = BTreeMap::new();
    for entry in archive.entries() {
        let extension = extension_of(entry);
        if !CHUNK_EXTENSIONS.contains(&extension.as_str()) && entry.uncompressed_size() >= 16 {
            by_extension.entry(extension).or_default().push(entry);
        }
    }
    by_extension
        .into_par_iter()
        .filter_map(|(extension, entries)| {
            let step = (entries.len() / SNIFFED_PER_EXTENSION).max(1);
            let (mut sampled, mut chunk_files) = (0, 0);
            for entry in entries
                .into_iter()
                .step_by(step)
                .take(SNIFFED_PER_EXTENSION)
            {
                let mut magic = [0_u8; 4];
                if archive
                    .open_entry(entry)
                    .ok()
                    .and_then(|mut reader| reader.read_exact(&mut magic).ok())
                    .is_some()
                {
                    sampled += 1;
                    chunk_files += usize::from(&magic == b"#ivo" || &magic == b"CrCh");
                }
            }
            (chunk_files > 0).then_some((extension, (chunk_files, sampled)))
        })
        .collect()
}

fn render(survey: &Survey) -> String {
    let mut out = String::from(
        "container\tid\tversion\tname\tchunks\tfiles\tin_socpak\tmin_size\tmax_size\textensions\texample\n",
    );
    for (&(crch, id, version), stat) in &survey.stats {
        let container = if crch == 1 {
            Container::CrCh
        } else {
            Container::Ivo
        };
        let kind = ChunkType::new(container, id);
        let extensions: Vec<String> = stat
            .extensions
            .iter()
            .map(|(extension, count)| format!("{extension}:{count}"))
            .collect();
        let _written = writeln!(
            out,
            "{container}\t{}\t0x{version:X}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            kind.hex_id(),
            kind.name().unwrap_or("UNKNOWN"),
            stat.chunks,
            stat.files,
            stat.in_container,
            stat.min_size,
            stat.max_size,
            extensions.join(","),
            stat.example
        );
    }
    out
}

#[test]
#[expect(
    clippy::print_stdout,
    reason = "the survey table is the measurement; the test prints it"
)]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_chunk_type_of_the_archive_is_counted_and_the_unknown_ones_are_listed() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let mut survey = scan_archive(&archive, "");
    let socpaks: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| extension_of(entry) == "socpak")
        .collect();
    let from_containers = socpaks
        .par_iter()
        .map(|entry| scan_container(&archive, entry))
        .reduce(Survey::default, Survey::merge);
    survey = survey.merge(from_containers);

    let table = render(&survey);
    println!("{table}");
    println!(
        "chunk files: {}, socpaks: {}, skipped: {:?}, failed: {} {:?}",
        survey.files,
        socpaks.len(),
        survey.skipped,
        survey.failed.len(),
        survey.failed.iter().take(10).collect::<Vec<_>>()
    );
    println!(
        "other extensions holding chunk files: {:?}",
        sniff_extensions(&archive)
    );
    println!("profiles of the disputed types:");
    for (text, count) in &survey.profiles {
        println!("{count}\t{text}");
    }
    println!("UNKNOWN:");
    for line in table.lines().filter(|line| line.contains("\tUNKNOWN\t")) {
        println!("{line}");
    }
    if let Some(out) = std::env::var_os("P4KX_SURVEY_OUT") {
        std::fs::write(out, &table)?;
    }
    assert!(survey.files > 0, "the Archive holds chunk files");
    assert!(survey.failed.is_empty(), "failed: {:#?}", survey.failed);
    Ok(())
}
