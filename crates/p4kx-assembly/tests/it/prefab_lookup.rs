// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Finding prefabs and libraries: dotted names, GUIDs, library names and unreadable libraries.

use p4kx_assembly::{PrefabError, PrefabIssueKind, PrefabSet};
use p4kx_testkit::{TestEntry, TestPrefabLibrary, TestPrefabObject, prefab_guid};

use crate::assembly_support::TestResult;
use crate::prefab::{IDENTITY, brush, kinds, nested, resolve, with_archive};

#[test]
fn a_prefab_name_may_contain_dots_and_the_guid_is_checked() -> TestResult {
    let library = TestPrefabLibrary::new("test/rooms")
        .prefab(
            "hall",
            vec![
                nested("good", "visual.flare", [0.0; 3], IDENTITY),
                nested("stale", "visual.flare", [0.0; 3], IDENTITY)
                    .attribute("PrefabGUID", "{00000000-0000-0000-0000-000000000001}"),
            ],
        )
        .prefab("visual.flare", vec![brush("glow", [0.0; 3])]);
    let resolved = resolve(&library, "HALL")?;
    assert_eq!(resolved.objects.len(), 2);
    assert_eq!(
        kinds(&resolved),
        [&PrefabIssueKind::GuidMismatch {
            prefab: "test/rooms.visual.flare".to_owned(),
            named: "{00000000-0000-0000-0000-000000000001}".to_owned(),
            found: prefab_guid("test/rooms", "visual.flare"),
        }]
    );
    Ok(())
}

#[test]
fn a_prefab_that_moved_is_found_by_its_guid() -> TestResult {
    let rooms = TestPrefabLibrary::new("test/rooms").prefab(
        "hall",
        vec![
            TestPrefabObject::nested("moved", "test/props", "part", [2.0, 0.0, 0.0], IDENTITY)
                .attribute("PrefabName", "gone/library.part"),
            TestPrefabObject::nested("wrong", "test/props", "part", [3.0, 0.0, 0.0], IDENTITY)
                .attribute("PrefabName", "test/rooms.hall"),
        ],
    );
    let props =
        TestPrefabLibrary::new("test/props").prefab("part", vec![brush("bolt", [1.0, 0.0, 0.0])]);
    let resolved = with_archive(vec![rooms.entry(), props.entry()], |archive| {
        Ok(PrefabSet::new(archive).resolve("test/rooms", "hall")?)
    })?;
    assert_eq!(resolved.issues, []);
    let bolts: Vec<[f32; 3]> = resolved
        .objects
        .iter()
        .map(|o| o.world().translation())
        .collect();
    assert_eq!(bolts, [[3.0, 0.0, 0.0], [4.0, 0.0, 0.0]]);
    Ok(())
}

#[test]
fn libraries_are_listed_and_found_by_name_or_path_ignoring_case() -> TestResult {
    let rooms = TestPrefabLibrary::new("test/Rooms").prefab("hall", vec![brush("wall", [0.0; 3])]);
    let props = TestPrefabLibrary::new("test/props").prefab("a", vec![]);
    with_archive(vec![props.entry(), rooms.entry()], |archive| {
        let mut set = PrefabSet::new(archive);
        assert_eq!(
            set.libraries().collect::<Vec<_>>(),
            ["test/props", "test/Rooms"]
        );
        for name in [
            "test/rooms",
            "TEST\\ROOMS",
            "Data\\Prefabs\\test\\rooms.xml",
        ] {
            let file = set.library(name)?;
            assert_eq!(file.library().prefabs.len(), 1);
        }
        assert!(matches!(
            set.library("test/none"),
            Err(PrefabError::NoSuchLibrary { .. })
        ));
        assert!(matches!(
            set.resolve("test/rooms", "none"),
            Err(PrefabError::NoSuchPrefab { .. })
        ));
        Ok(())
    })
}

#[test]
fn a_library_that_cannot_be_read_is_an_error_and_an_issue_for_its_users() -> TestResult {
    let rooms = TestPrefabLibrary::new("test/rooms").prefab(
        "hall",
        vec![TestPrefabObject::nested(
            "x",
            "test/broken",
            "p",
            [0.0; 3],
            IDENTITY,
        )],
    );
    let broken = TestEntry::stored(
        "Data\\Prefabs\\test\\broken.xml",
        b"<ObjectContainer/>".to_vec(),
    );
    with_archive(vec![rooms.entry(), broken], |archive| {
        let mut set = PrefabSet::new(archive);
        assert!(matches!(
            set.library("test/broken"),
            Err(PrefabError::Unreadable { .. })
        ));
        let resolved = set.resolve("test/rooms", "hall")?;
        assert_eq!(resolved.objects.len(), 1);
        assert!(matches!(
            kinds(&resolved)[..],
            [
                PrefabIssueKind::LibraryUnreadable { .. },
                PrefabIssueKind::Missing { .. }
            ]
        ));
        Ok(())
    })
}
