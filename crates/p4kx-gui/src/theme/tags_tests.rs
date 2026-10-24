// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde_json::Value;

use super::desktop as d;
use super::tags_metrics as t;

const DESKTOP: &str = include_str!("../../../../docs/design/p4kx-desktop-spec.json");

fn spec() -> Value {
    serde_json::from_str(DESKTOP).unwrap_or_else(|error| panic!("the desktop spec: {error}"))
}

#[test]
fn the_tag_tree_metrics_match_the_desktop_spec() {
    let json = spec();

    for (key, value) in [
        ("guidColumnWidth", t::TAGS_GUID_WIDTH),
        ("legacyColumnWidth", t::TAGS_LEGACY_WIDTH),
        ("nameColumnMinWidth", t::TAGS_NAME_MIN_WIDTH),
        ("rowHeight", d::ROW_HEIGHT),
    ] {
        let expected = json["tagTree"][key].as_f64();
        assert!(
            expected.is_some_and(|want| (want - f64::from(value)).abs() < 0.01),
            "tagTree.{key} is {expected:?}, the theme has {value}"
        );
    }
    assert_eq!(json["tagTree"]["decision"], "D.45", "the decision");
}
