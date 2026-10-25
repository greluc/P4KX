// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! How a refusal names the installation, and the folders entered by hand.

use std::error::Error;
use std::fs;
use std::path::Path;

use p4kx_guard::{Installation, ensure_writable, protect_manual, without_verbatim_prefix};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn the_verbatim_prefix_is_dropped_from_a_shown_path() {
    assert_eq!(
        without_verbatim_prefix(Path::new(r"\\?\C:\Games\StarCitizen")),
        Path::new(r"C:\Games\StarCitizen")
    );
    assert_eq!(
        without_verbatim_prefix(Path::new(r"\\?\UNC\server\share\StarCitizen")),
        Path::new(r"\\server\share\StarCitizen")
    );
    assert_eq!(
        without_verbatim_prefix(Path::new(r"C:\Games")),
        Path::new(r"C:\Games")
    );
    assert_eq!(
        without_verbatim_prefix(Path::new(r"\\?\Volume{1234}\x")),
        Path::new(r"\\?\Volume{1234}\x"),
        "a volume path has no plain form"
    );
    assert_eq!(
        without_verbatim_prefix(Path::new("/games/StarCitizen")),
        Path::new("/games/StarCitizen")
    );
}

#[test]
fn a_refusal_names_the_installation_without_the_verbatim_prefix() -> TestResult {
    let root = tempfile::tempdir()?;
    let folder = root.path().join("StarCitizen");
    fs::create_dir_all(&folder)?;
    let mut installation = Installation::new();
    installation.add(&folder);

    let refused = installation
        .check(&folder.join("LIVE"))
        .err()
        .ok_or("the target was allowed")?;

    assert!(
        !refused.installation.to_string_lossy().starts_with(r"\\?\"),
        "{}",
        refused.installation.display()
    );
    assert!(!refused.to_string().contains(r"\\?\"), "{refused}");
    Ok(())
}

#[test]
fn folders_entered_by_hand_are_protected_and_archives_only_with_bin64() -> TestResult {
    let root = tempfile::tempdir()?;
    let library = root.path().join("ManualLibrary");
    let channel = root.path().join("ManualChannel");
    let installed = root.path().join("Installed");
    let copy = root.path().join("Copy");
    let workspace = root.path().join("Workspace");
    for folder in [
        &library.join("LIVE").join("Bin64"),
        &channel.join("Bin64"),
        &installed.join("Bin64"),
        &copy,
        &workspace.join("notes"),
    ] {
        fs::create_dir_all(folder)?;
    }
    fs::write(channel.join("Data.p4k"), b"")?;
    fs::write(installed.join("Data.p4k"), b"")?;
    fs::write(copy.join("Data.p4k"), b"")?;

    protect_manual(
        &[library.clone(), channel.clone(), workspace.clone()],
        &[installed.join("Data.p4k"), copy.join("Data.p4k")],
    );

    assert!(ensure_writable(&library.join("LIVE").join("out")).is_err());
    assert!(ensure_writable(&channel.join("export")).is_err());
    assert!(
        ensure_writable(&workspace.join("export")).is_ok(),
        "a manual folder without Bin64 stays writable"
    );
    assert!(ensure_writable(&installed.join("export")).is_err());
    assert!(
        ensure_writable(&copy.join("export")).is_ok(),
        "a copy of an Archive is not an installation"
    );
    assert!(ensure_writable(&root.path().join("elsewhere")).is_ok());
    Ok(())
}
