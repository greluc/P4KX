//! `p4kx extract` end to end.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, TestEntry, write_build_manifest};
use predicates::prelude::predicate;

type TestResult = Result<(), Box<dyn Error>>;

fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let archive = dir.join("Data.p4k");
    let zstd = TestEntry::zstd("Data\\broken-secret.xml", vec![b'x'; 4096]);
    let mut broken_zstd = zstd.payload().get(..4).unwrap_or_default().to_vec();
    broken_zstd.extend_from_slice(&[0xFF; 64]);
    ArchiveBuilder::new()
        .entry(TestEntry::stored(
            "Data\\Libs\\Config\\defaultProfile.xml",
            b"<Profile/>".to_vec(),
        ))
        .entry(TestEntry::stored("Data\\readme.txt", b"hello".to_vec()))
        .entry(TestEntry::stored("..\\..\\escaped.txt", b"evil".to_vec()))
        .entry(TestEntry::stored("C:\\drive.txt", b"evil".to_vec()))
        .entry(TestEntry::deflated(
            "Data\\deflate.xml",
            b"<deflate/>".to_vec(),
        ))
        .entry(TestEntry::zstd("Data\\zstd.xml", b"<zstd/>".to_vec()))
        .entry(TestEntry::zstd_without_magic(
            "Data\\plain.xml",
            b"<plain/>".to_vec(),
        ))
        .entry(TestEntry::zstd("Data\\secret.xml", b"<secret/>".to_vec()).encrypted())
        .entry(zstd.with_payload(broken_zstd).encrypted())
        .entry(
            TestEntry::deflated("Data\\broken.xml", b"<broken/>".to_vec())
                .with_payload(vec![0xFF; 8]),
        )
        .write_to(&archive)?;
    Ok(archive)
}

fn export_folders(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut names = Vec::new();
    for item in std::fs::read_dir(dir)? {
        let name = item?.file_name().to_string_lossy().into_owned();
        if name.starts_with("SC-EXPORT-") {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

#[test]
fn extract_writes_the_requested_entries_into_out() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("out");

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .arg("Data/Libs/Config/defaultProfile.xml")
        .arg("Data\\readme.txt")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "extracted 2, skipped 0, failed 0, cancelled 0",
        ));

    assert_eq!(
        std::fs::read(out.join("Data/Libs/Config/defaultProfile.xml"))?,
        b"<Profile/>",
        "nested Entry content"
    );
    assert_eq!(
        std::fs::read(out.join("Data/readme.txt"))?,
        b"hello",
        "Entry content"
    );
    Ok(())
}

#[test]
fn the_default_target_is_named_after_the_channel_and_the_build_version() -> TestResult {
    let dir = tempfile::tempdir()?;
    let channel = dir.path().join("PTU");
    std::fs::create_dir(&channel)?;
    let archive = write_archive(&channel)?;
    write_build_manifest(
        &channel,
        "sc-alpha-4.11.0",
        "4.11.42.1234",
        "12999999",
        "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0",
    )?;

    p4kx_testkit::p4kx()?
        .current_dir(dir.path())
        .arg("extract")
        .arg(&archive)
        .arg("Data\\readme.txt")
        .assert()
        .code(0);

    let folders = export_folders(dir.path())?;
    assert_eq!(folders.len(), 1, "one folder: {folders:?}");
    assert!(
        folders
            .iter()
            .all(|folder| folder.starts_with("SC-EXPORT-PTU-4.11.42.1234-")),
        "Channel and Build version in the name: {folders:?}"
    );
    Ok(())
}

#[test]
fn extract_without_out_creates_a_new_default_target_for_every_run() -> TestResult {
    let dir = tempfile::tempdir()?;
    let channel = dir.path().join("LIVE");
    std::fs::create_dir(&channel)?;
    let archive = write_archive(&channel)?;

    for _ in 0..2 {
        p4kx_testkit::p4kx()?
            .current_dir(dir.path())
            .arg("extract")
            .arg(&archive)
            .arg("Data\\readme.txt")
            .assert()
            .code(0);
    }

    let folders = export_folders(dir.path())?;
    assert_eq!(folders.len(), 2, "two runs, two folders: {folders:?}");
    for folder in &folders {
        assert!(
            folder.starts_with("SC-EXPORT-LIVE-UnknownBuild-"),
            "default name pattern: {folder}"
        );
        assert_eq!(
            std::fs::read(dir.path().join(folder).join("Data/readme.txt"))?,
            b"hello",
            "each run extracts into its own folder"
        );
    }
    Ok(())
}

#[test]
fn extract_rejects_entry_paths_outside_the_target_with_exit_code_1() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("a").join("b").join("out");

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .arg("..\\..\\escaped.txt")
        .arg("C:\\drive.txt")
        .arg("Data\\readme.txt")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "extracted 1, skipped 0, failed 2, cancelled 0",
        ))
        .stderr(predicate::str::contains(
            "failed ..\\..\\escaped.txt: the path would leave the target folder",
        ))
        .stderr(predicate::str::contains(
            "failed C:\\drive.txt: the path would leave the target folder",
        ));

    assert!(
        !dir.path().join("a").join("escaped.txt").exists()
            && !dir.path().join("escaped.txt").exists(),
        "nothing is written outside the target"
    );
    assert_eq!(
        std::fs::read_dir(&out)?.count(),
        1,
        "only the Data folder is in the target"
    );
    Ok(())
}

#[test]
fn extract_writes_compressed_and_encrypted_entries_with_their_original_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("out");
    let expected: [(&str, &[u8]); 4] = [
        ("Data/deflate.xml", b"<deflate/>"),
        ("Data/zstd.xml", b"<zstd/>"),
        ("Data/plain.xml", b"<plain/>"),
        ("Data/secret.xml", b"<secret/>"),
    ];

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .args(expected.iter().map(|(path, _)| path))
        .arg("--out")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "extracted 4, skipped 0, failed 0, cancelled 0",
        ));

    for (path, data) in expected {
        assert_eq!(std::fs::read(out.join(path))?, data, "content of {path}");
    }
    Ok(())
}

#[test]
fn extract_writes_the_entries_of_a_v2_archive() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    let out = dir.path().join("out");
    ArchiveBuilder::v2()
        .entry(TestEntry::stored("Data\\readme.txt", b"hello".to_vec()))
        .entry(TestEntry::zstd("Data\\secret.xml", b"<secret/>".to_vec()).encrypted())
        .write_to(&archive)?;

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .arg("Data\\readme.txt")
        .arg("Data\\secret.xml")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "extracted 2, skipped 0, failed 0, cancelled 0",
        ));

    assert_eq!(
        std::fs::read(out.join("Data/readme.txt"))?,
        b"hello",
        "stored Entry"
    );
    assert_eq!(
        std::fs::read(out.join("Data/secret.xml"))?,
        b"<secret/>",
        "encrypted zstd Entry"
    );
    Ok(())
}

#[test]
fn extract_of_corrupt_entries_fails_only_those_entries_with_exit_code_1() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("out");

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .arg("Data\\broken.xml")
        .arg("Data\\broken-secret.xml")
        .arg("Data\\readme.txt")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "extracted 1, skipped 0, failed 2, cancelled 0",
        ))
        .stderr(predicate::str::contains(
            "failed Data\\broken.xml: cannot read the Entry data: deflate data is corrupt",
        ))
        .stderr(predicate::str::contains(
            "failed Data\\broken-secret.xml: cannot read the Entry data: zstd data is corrupt",
        ));

    assert!(
        !out.join("Data/broken.xml").exists() && !out.join("Data/broken-secret.xml").exists(),
        "no file for the corrupt Entries"
    );
    assert_eq!(
        std::fs::read(out.join("Data/readme.txt"))?,
        b"hello",
        "the other Entry is extracted"
    );
    Ok(())
}

#[test]
fn extract_keeps_existing_files_and_reports_them_as_skipped() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("out");
    std::fs::create_dir_all(out.join("Data"))?;
    std::fs::write(out.join("Data/readme.txt"), "mine")?;

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .arg("Data\\readme.txt")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(0)
        .stdout(predicate::str::contains(
            "skipped Data\\readme.txt: already exists",
        ))
        .stdout(predicate::str::contains(
            "extracted 0, skipped 1, failed 0, cancelled 0",
        ));

    assert_eq!(
        std::fs::read_to_string(out.join("Data/readme.txt"))?,
        "mine",
        "the existing file is unchanged"
    );
    Ok(())
}

#[test]
fn extract_of_an_unknown_entry_is_invalid_input_with_exit_code_2() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    p4kx_testkit::p4kx()?
        .current_dir(dir.path())
        .arg("extract")
        .arg(&archive)
        .arg("Data\\readme.txt")
        .arg("Data\\missing.xml")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no Entry named Data\\missing.xml"));

    assert!(
        export_folders(dir.path())?.is_empty(),
        "no target is created for invalid input"
    );
    Ok(())
}

#[test]
fn extract_without_entry_paths_is_invalid_input_with_exit_code_2() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .assert()
        .code(2);
    Ok(())
}

#[test]
fn extract_from_a_file_that_is_not_an_archive_exits_with_code_3() -> TestResult {
    let dir = tempfile::tempdir()?;
    let not_archive = dir.path().join("notes.txt");
    std::fs::write(&not_archive, "plain text")?;

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&not_archive)
        .arg("Data\\readme.txt")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("is not a p4k Archive"));
    Ok(())
}

#[test]
fn extract_help_documents_the_exit_codes() -> TestResult {
    p4kx_testkit::p4kx()?
        .args(["extract", "--help"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Exit codes:"))
        .stdout(predicate::str::contains("2  invalid input"))
        .stdout(predicate::str::contains(
            "3  the Archive or the input file cannot be read",
        ));
    Ok(())
}
