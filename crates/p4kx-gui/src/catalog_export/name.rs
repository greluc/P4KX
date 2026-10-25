// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::ffi::OsString;
use std::path::{Path, PathBuf};

const JSON: &str = ".json";
const ZST: &str = ".zst";

fn ends_with_ignoring_case(text: &str, suffix: &str) -> bool {
    text.len() >= suffix.len()
        && text
            .get(text.len() - suffix.len()..)
            .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
}

/// Returns `file` as the catalog is written: a doubled `.json` that a save dialog appends to a
/// typed `.json` is collapsed, and a name ending in `.zst` without `.json` before it gets one, so
/// `foo.zst` becomes `foo.json.zst`. Any other name, and a name that is not text, stays as it is.
pub(crate) fn normalised_catalog_file(file: PathBuf) -> PathBuf {
    let Some(name) = file.file_name().and_then(|name| name.to_str()) else {
        return file;
    };
    let (mut stem, packed) = if ends_with_ignoring_case(name, ZST) {
        name.split_at(name.len() - ZST.len())
    } else {
        (name, "")
    };
    let mut doubled = false;
    while ends_with_ignoring_case(stem, ".json.json") {
        stem = stem.get(..stem.len() - JSON.len()).unwrap_or(stem);
        doubled = true;
    }
    let mut fixed = stem.to_owned();
    if !packed.is_empty() && !ends_with_ignoring_case(stem, JSON) {
        fixed.push_str(JSON);
        doubled = true;
    }
    if !doubled {
        return file;
    }
    fixed.push_str(packed);
    file.with_file_name(OsString::from(fixed))
}

/// Returns whether `file` ends in `.gz`, ignoring case.
pub(crate) fn is_gzip(file: &Path) -> bool {
    file.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gz"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalised(name: &str) -> PathBuf {
        normalised_catalog_file(Path::new("out").join(name))
    }

    #[test]
    fn a_zst_name_without_json_gets_json_before_it() {
        assert_eq!(normalised("foo.zst"), Path::new("out/foo.json.zst"));
        assert_eq!(normalised("foo.ZST"), Path::new("out/foo.json.ZST"));
    }

    #[test]
    fn a_doubled_json_is_collapsed() {
        assert_eq!(normalised("foo.json.json"), Path::new("out/foo.json"));
        assert_eq!(
            normalised("foo.json.json.zst"),
            Path::new("out/foo.json.zst")
        );
        assert_eq!(normalised("foo.JSON.json"), Path::new("out/foo.JSON"));
        assert_eq!(normalised("foo.json.json.json"), Path::new("out/foo.json"));
    }

    #[test]
    fn a_name_that_is_right_stays_as_it_is() {
        for name in ["foo.json.zst", "foo.json", "foo", "foo.txt", "foo.json.gz"] {
            assert_eq!(normalised(name), Path::new("out").join(name), "{name}");
        }
    }

    #[test]
    fn only_the_file_name_changes() {
        assert_eq!(
            normalised_catalog_file(PathBuf::from("a.zst/b.zst")),
            Path::new("a.zst/b.json.zst")
        );
    }

    #[test]
    fn a_gz_name_is_gzip_in_any_case() {
        assert!(is_gzip(Path::new("foo.json.gz")));
        assert!(is_gzip(Path::new("foo.GZ")));
        assert!(!is_gzip(Path::new("foo.json.zst")));
        assert!(!is_gzip(Path::new("gz")));
    }
}
