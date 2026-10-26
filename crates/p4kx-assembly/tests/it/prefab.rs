// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Prefab libraries of a synthetic Archive: nested prefabs resolved with composed transforms,
//! cycles, missing prefabs and the limits.

use p4kx_assembly::{
    MAX_PREFAB_DEPTH, MAX_PREFAB_OBJECTS, PrefabIssueKind, PrefabSet, ResolvedObject,
    ResolvedPrefab,
};
use p4kx_extraction::Archive;
use p4kx_testkit::{TestEntry, TestPrefabLibrary, TestPrefabObject};

use crate::assembly_support::{TestResult, fixture};

pub(crate) const IDENTITY: [f64; 4] = [1.0, 0.0, 0.0, 0.0];
const QUARTER_Z: [f64; 4] = [
    std::f64::consts::FRAC_1_SQRT_2,
    0.0,
    0.0,
    std::f64::consts::FRAC_1_SQRT_2,
];

pub(crate) fn brush(name: &str, at: [f64; 3]) -> TestPrefabObject {
    TestPrefabObject::brush(name, &format!("objects/test/{name}.cgf"), at, IDENTITY)
}

pub(crate) fn nested(
    name: &str,
    prefab: &str,
    at: [f64; 3],
    rotation: [f64; 4],
) -> TestPrefabObject {
    TestPrefabObject::nested(name, "test/rooms", prefab, at, rotation)
}

fn near(actual: [f32; 3], expected: [f32; 3]) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (a - e).abs() < 1e-4)
}

pub(crate) fn with_archive<T>(
    entries: Vec<TestEntry>,
    check: impl FnOnce(&Archive) -> Result<T, Box<dyn std::error::Error>>,
) -> Result<T, Box<dyn std::error::Error>> {
    let fixture = fixture(entries)?;
    check(&Archive::open(&fixture.path)?)
}

pub(crate) fn resolve(
    library: &TestPrefabLibrary,
    prefab: &str,
) -> Result<ResolvedPrefab, Box<dyn std::error::Error>> {
    with_archive(vec![library.entry()], |archive| {
        Ok(PrefabSet::new(archive).resolve("test/rooms", prefab)?)
    })
}

fn object<'a>(resolved: &'a ResolvedPrefab, name: &str) -> Result<&'a ResolvedObject, String> {
    resolved
        .objects
        .iter()
        .find(|object| object.name() == name)
        .ok_or_else(|| format!("no object {name}"))
}

pub(crate) fn kinds(resolved: &ResolvedPrefab) -> Vec<&PrefabIssueKind> {
    resolved.issues.iter().map(|issue| &issue.kind).collect()
}

#[test]
fn a_nested_prefab_is_expanded_with_the_parents_rotation_applied_to_its_positions() -> TestResult {
    let library = TestPrefabLibrary::new("test/rooms")
        .prefab(
            "hall",
            vec![
                brush("wall", [1.0, 2.0, 3.0]),
                nested("sub", "closet", [10.0, 0.0, 0.0], QUARTER_Z),
            ],
        )
        .prefab("closet", vec![brush("shelf", [1.0, 0.0, 0.0])]);
    let resolved = resolve(&library, "hall")?;
    assert_eq!(resolved.issues, []);
    assert_eq!(resolved.prefabs, 2);
    assert_eq!(resolved.objects.len(), 2);
    let wall = object(&resolved, "wall")?;
    assert!(near(wall.world().translation(), [1.0, 2.0, 3.0]));
    assert_eq!((wall.container(), wall.depth()), ("", 0));
    let shelf = object(&resolved, "shelf")?;
    assert!(near(shelf.world().translation(), [10.0, 1.0, 0.0]));
    assert_eq!((shelf.container(), shelf.depth()), ("sub", 1));
    assert_eq!((shelf.prefab(), shelf.library()), ("closet", "test/rooms"));
    assert_eq!(shelf.model(), Some("objects/test/shelf.cgf"));
    Ok(())
}

#[test]
fn rotations_and_scales_compose_over_three_levels() -> TestResult {
    let library = TestPrefabLibrary::new("test/rooms")
        .prefab(
            "top",
            vec![nested("mid", "middle", [0.0, 0.0, 5.0], QUARTER_Z).scaled([2.0, 2.0, 2.0])],
        )
        .prefab(
            "middle",
            vec![nested("low", "bottom", [1.0, 0.0, 0.0], QUARTER_Z)],
        )
        .prefab("bottom", vec![brush("leaf", [1.0, 0.0, 0.0])]);
    let resolved = resolve(&library, "top")?;
    let leaf = object(&resolved, "leaf")?;
    assert_eq!(leaf.container(), "mid/low");
    assert_eq!(leaf.depth(), 2);
    assert!(near(leaf.world().translation(), [-2.0, 2.0, 5.0]));
    let rows = leaf.world().rows;
    assert!((rows[0][0] + 2.0).abs() < 1e-4 && rows[1][0].abs() < 1e-4);
    Ok(())
}

#[test]
fn prefabs_that_contain_each_other_are_expanded_once_and_reported() -> TestResult {
    let library = TestPrefabLibrary::new("test/rooms")
        .prefab(
            "loop_a",
            vec![
                brush("a", [0.0; 3]),
                nested("to_b", "loop_b", [1.0, 0.0, 0.0], IDENTITY),
            ],
        )
        .prefab(
            "loop_b",
            vec![
                brush("b", [0.0; 3]),
                nested("to_a", "loop_a", [1.0, 0.0, 0.0], IDENTITY),
            ],
        )
        .prefab("selfish", vec![nested("me", "selfish", [0.0; 3], IDENTITY)]);
    let resolved = resolve(&library, "loop_a")?;
    assert_eq!(resolved.objects.len(), 3);
    assert!(near(
        object(&resolved, "b")?.world().translation(),
        [1.0, 0.0, 0.0]
    ));
    let back = object(&resolved, "to_a")?;
    assert_eq!(
        back.unexpanded().map(|r| r.name.as_str()),
        Some("test/rooms.loop_a")
    );
    assert!(matches!(
        kinds(&resolved)[..],
        [PrefabIssueKind::Cycle { .. }]
    ));
    let issue = resolved.issues.first().ok_or("no issue")?;
    assert_eq!(
        (issue.container.as_str(), issue.name.as_str()),
        ("to_b", "to_a")
    );

    let selfish = resolve(&library, "selfish")?;
    assert_eq!(selfish.objects.len(), 1);
    assert!(matches!(
        kinds(&selfish)[..],
        [PrefabIssueKind::Cycle { .. }]
    ));
    Ok(())
}

#[test]
fn a_prefab_no_library_holds_stays_in_the_list_with_an_issue() -> TestResult {
    let library = TestPrefabLibrary::new("test/rooms").prefab(
        "hall",
        vec![
            nested("ghost", "nothing", [0.0; 3], IDENTITY),
            TestPrefabObject::nested("elsewhere", "no/such/library", "x", [0.0; 3], IDENTITY),
            TestPrefabObject::nested("blank", "", "", [0.0; 3], IDENTITY)
                .attribute("PrefabName", ""),
        ],
    );
    let resolved = resolve(&library, "hall")?;
    assert_eq!(resolved.objects.len(), 3);
    assert_eq!(
        kinds(&resolved),
        [
            &PrefabIssueKind::Missing {
                prefab: "test/rooms.nothing".to_owned()
            },
            &PrefabIssueKind::Missing {
                prefab: "no/such/library.x".to_owned()
            },
            &PrefabIssueKind::Missing {
                prefab: String::new()
            },
        ]
    );
    Ok(())
}

#[test]
fn the_chain_of_nested_prefabs_is_bounded() -> TestResult {
    let mut library = TestPrefabLibrary::new("test/rooms");
    let levels = MAX_PREFAB_DEPTH + 4;
    for level in 0..levels {
        let mut objects = vec![brush(&format!("leaf{level}"), [0.0; 3])];
        if level + 1 < levels {
            objects.push(nested(
                "next",
                &format!("d{}", level + 1),
                [1.0, 0.0, 0.0],
                IDENTITY,
            ));
        }
        library = library.prefab(format!("d{level}"), objects);
    }
    let resolved = resolve(&library, "d0")?;
    assert_eq!(resolved.prefabs, MAX_PREFAB_DEPTH);
    assert_eq!(
        kinds(&resolved),
        [&PrefabIssueKind::TooDeep {
            limit: MAX_PREFAB_DEPTH
        }]
    );
    let last = resolved.objects.last().ok_or("no objects")?;
    assert_eq!(last.name(), "next");
    assert!(last.unexpanded().is_some());
    Ok(())
}

#[test]
fn a_fan_out_is_cut_at_the_object_limit() -> TestResult {
    let levels = 9;
    let mut library = TestPrefabLibrary::new("test/rooms");
    for level in 0..levels {
        let objects = if level + 1 == levels {
            vec![brush("leaf", [0.0; 3])]
        } else {
            (0..10)
                .map(|n| {
                    nested(
                        &format!("n{n}"),
                        &format!("f{}", level + 1),
                        [0.0; 3],
                        IDENTITY,
                    )
                })
                .collect()
        };
        library = library.prefab(format!("f{level}"), objects);
    }
    let resolved = resolve(&library, "f0")?;
    assert_eq!(resolved.objects.len(), MAX_PREFAB_OBJECTS);
    assert!(
        kinds(&resolved).contains(&&PrefabIssueKind::TooManyObjects {
            limit: MAX_PREFAB_OBJECTS
        })
    );
    Ok(())
}

#[test]
fn objects_that_cannot_be_placed_and_attached_ones_are_reported_once() -> TestResult {
    let library = TestPrefabLibrary::new("test/rooms")
        .prefab(
            "hall",
            vec![
                nested("one", "bits", [0.0; 3], IDENTITY),
                nested("two", "bits", [0.0; 3], IDENTITY),
            ],
        )
        .prefab(
            "bits",
            vec![
                brush("bad", [0.0; 3]).rotate_text("0,0,0,0"),
                brush("hung", [0.0; 3]).attribute("Parent", "{OTHER}"),
            ],
        );
    let resolved = resolve(&library, "hall")?;
    assert_eq!(resolved.objects.len(), 2);
    assert!(resolved.objects.iter().all(ResolvedObject::attached));
    assert_eq!(
        kinds(&resolved),
        [
            &PrefabIssueKind::ObjectsLeftOut {
                prefab: "bits".to_owned(),
                count: 1
            },
            &PrefabIssueKind::Attached {
                prefab: "bits".to_owned(),
                count: 1
            },
        ]
    );
    Ok(())
}
