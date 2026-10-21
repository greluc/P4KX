//! The size cap of the Model tab covers every file reading a model fetches.

mod headless;
mod viewing;

use headless::{opened, ready, status_is_ready};
use p4kx_gui::{Language, MODEL_FILE_LIMIT, ModelStatus};
use p4kx_testkit::{TestEntry, TestModel};
use viewing::{Site, TestResult, model_preview, opened_at, select};

fn too_large_with(sibling: &str) -> Result<Option<u64>, Box<dyn std::error::Error>> {
    const GEOMETRY: &str = "Data\\Objects\\cube.cgfm";
    let cube = TestModel::cube();
    let mut entries = vec![TestEntry::zstd("Data\\Objects\\cube.cgf", cube.cgf())];
    if sibling != GEOMETRY {
        entries.push(TestEntry::zstd(GEOMETRY, cube.cgfm()));
    }
    entries
        .push(TestEntry::stored(sibling, "not read").with_uncompressed_size(MODEL_FILE_LIMIT + 1));
    let site = Site {
        entries,
        installation: false,
        key: false,
    };
    let (_folder, mut harness) = opened_at(Language::BritishEnglish, ready(), site, |b| b)?;
    select(&mut harness, "cube.cgf")?;
    Ok(
        model_preview(harness.state()).and_then(|preview| match preview.status() {
            ModelStatus::FileTooLarge(bytes) => Some(bytes),
            _ => None,
        }),
    )
}

#[test]
fn a_geometry_companion_above_the_limit_is_not_read() -> TestResult {
    assert_eq!(
        too_large_with("Data\\Objects\\cube.cgfm")?,
        Some(MODEL_FILE_LIMIT + 1),
        "named"
    );
    Ok(())
}

#[test]
fn a_level_of_detail_companion_above_the_limit_is_not_read() -> TestResult {
    for sibling in [
        "Data\\Objects\\cube_lod3.cgfm",
        "Data\\Objects\\cube_lod15.cgf",
    ] {
        assert_eq!(
            too_large_with(sibling)?,
            Some(MODEL_FILE_LIMIT + 1),
            "{sibling}"
        );
    }
    Ok(())
}

#[test]
fn a_model_whose_companions_are_small_is_read() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, ready())?;

    select(&mut harness, "cube.cgf")?;

    assert!(status_is_ready(&harness), "read");
    Ok(())
}
