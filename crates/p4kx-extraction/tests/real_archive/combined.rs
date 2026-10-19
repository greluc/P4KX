//! The combined Record export and the DataForge Conversion over the live DataForge.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use p4kx_extraction::{
    Archive, DataForgeEntry, ExtractOptions, RECORD_XML_NAMESPACE, RecordExportOptions,
    export_combined, extract,
};
use p4kx_testkit::real_archive_path;

use crate::TestResult;

const KNOWN_RECORD: &str = "libs/foundry/records/entities/spaceships/aegs_avenger_titan";

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_combined_document_of_a_real_archive_parses() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let records: Vec<_> = forge.records().collect();
    let target = tempfile::tempdir()?;
    let file = target.path().join("Game2.xml");

    let report = export_combined(&forge, &records, &file, &RecordExportOptions::default())?;

    assert!(report.is_complete(), "{:?}", report.failures());
    assert_eq!(report.written(), forge.record_count(), "every Record");
    let (paths, guids) = check_document(&file)?;
    assert_eq!(paths.len(), forge.record_count(), "one element per Record");
    let expected: HashSet<String> = forge
        .records()
        .map(|record| record.guid().to_string())
        .collect();
    assert_eq!(
        guids.iter().cloned().collect::<HashSet<_>>(),
        expected,
        "the document holds every Record of the DataForge, none twice"
    );
    let folded: Vec<String> = paths
        .iter()
        .map(|path| path.to_lowercase().replace('\\', "/"))
        .collect();
    assert!(
        folded.windows(2).all(|pair| pair[0] <= pair[1]),
        "record-path order"
    );
    Ok(())
}

/// The record paths and the GUIDs of the Records of a combined document, in document order.
type Listed = (Vec<String>, Vec<String>);

/// Parses the combined document at `file` one Record element at a time and returns the record
/// path and the GUID of each, so the whole document is checked without holding it in memory.
fn check_document(file: &Path) -> Result<Listed, Box<dyn std::error::Error>> {
    let mut lines = BufReader::new(File::open(file)?).lines();
    assert_eq!(
        lines.next().transpose()?.as_deref(),
        Some("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"),
        "declaration"
    );
    let head = format!("<p4kx:records xmlns:p4kx=\"{RECORD_XML_NAMESPACE}\">");
    assert_eq!(
        lines.next().transpose()?,
        Some(head.clone()),
        "root element"
    );
    let mut paths = Vec::new();
    let mut guids = Vec::new();
    let mut chunk = String::new();
    let mut closed = false;
    for line in lines {
        let line = line?;
        assert!(!closed, "nothing follows the root element: {line}");
        let starts_record = line.starts_with("  <") && !line.starts_with("   ");
        let is_end = line == "</p4kx:records>";
        if (starts_record && !line.starts_with("  </") || is_end) && !chunk.is_empty() {
            let (path, guid) = parse_record(&head, &chunk)?;
            paths.push(path);
            guids.push(guid);
            chunk.clear();
        }
        if is_end {
            closed = true;
        } else {
            chunk.push_str(&line);
            chunk.push('\n');
        }
    }
    assert!(closed, "the root element is closed");
    Ok((paths, guids))
}

fn parse_record(head: &str, chunk: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let wrapped = format!("{head}\n{chunk}</p4kx:records>\n");
    let document = roxmltree::Document::parse(&wrapped)?;
    let mut elements = document
        .root_element()
        .children()
        .filter(roxmltree::Node::is_element);
    let record = elements.next().ok_or("a Record element")?;
    assert!(elements.next().is_none(), "one Record per chunk: {chunk}");
    let path = record.attribute("path").ok_or("a record path")?;
    let guid = record.attribute("guid").ok_or("a GUID")?;
    Ok((path.to_owned(), guid.to_owned()))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn extracting_the_dataforge_entry_with_conversion_writes_every_record() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let entry = DataForgeEntry::locate(&archive).ok_or("the Archive holds a DataForge")?;
    let record_count = DataForgeEntry::read(&archive)?.open()?.record_count();
    let target = tempfile::tempdir()?;
    let options = ExtractOptions {
        convert: true,
        ..ExtractOptions::default()
    };

    let report = extract(&archive, &[entry], target.path(), &options);

    assert!(report.is_complete(), "{report:?}");
    assert_eq!(
        report.converted(),
        Some(1),
        "the DataForge Entry is converted"
    );
    let records = report.records().ok_or("the Records are reported")?;
    assert_eq!(records.written(), record_count, "every Record is written");
    assert!(
        !target.path().join("Data/Game2.dcb").exists(),
        "no raw .dcb"
    );
    let xml = std::fs::read_to_string(target.path().join(format!("Data/{KNOWN_RECORD}.xml")))?;
    let document = roxmltree::Document::parse(&xml)?;
    assert_eq!(
        document.root_element().attribute("path"),
        Some(KNOWN_RECORD),
        "the known Record"
    );
    Ok(())
}
