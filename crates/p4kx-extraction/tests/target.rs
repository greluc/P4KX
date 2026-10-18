//! Naming and creating the default target folder of an Extraction.

use std::error::Error;
use std::time::{Duration, SystemTime};

use p4kx_extraction::{create_default_target, default_target_name};

type TestResult = Result<(), Box<dyn Error>>;

fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
}

#[test]
fn the_default_name_holds_channel_build_and_utc_timestamp() {
    assert_eq!(
        default_target_name(Some("LIVE"), Some("4.3.2-live.10234567"), at(1_700_000_000)),
        "SC-EXPORT-LIVE-4.3.2-live.10234567-20231114T221320Z",
        "known Channel and Build"
    );
    assert_eq!(
        default_target_name(None, None, at(951_782_400)),
        "SC-EXPORT-UnknownChannel-UnknownBuild-20000229T000000Z",
        "unknown parts are named as such"
    );
}

#[test]
fn the_default_name_never_contains_path_separators() {
    let name = default_target_name(Some("a/b"), Some("c\\d:e"), at(0));

    assert_eq!(
        name, "SC-EXPORT-a_b-c_d_e-19700101T000000Z",
        "characters outside letters, digits, dot, dash and underscore are replaced"
    );
}

#[test]
fn runs_started_in_the_same_second_get_separate_folders() -> TestResult {
    let dir = tempfile::tempdir()?;

    let first = create_default_target(dir.path(), None, None, at(0))?;
    let second = create_default_target(dir.path(), None, None, at(0))?;

    assert!(first.is_dir() && second.is_dir(), "both folders exist");
    assert_ne!(first, second, "the second run gets its own folder");
    assert_eq!(
        second.file_name().and_then(|name| name.to_str()),
        Some("SC-EXPORT-UnknownChannel-UnknownBuild-19700101T000000Z-2"),
        "the second folder is numbered"
    );
    Ok(())
}
