// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The skeleton file a character definition names is checked: a missing, malformed or oversized
//! file is an Issue and the skin is still drawn.

use crate::assembly_support::{TestResult, assemble, fixture};
use crate::character_definitions::{CHARACTER, entries};

use p4kx_assembly::{
    ArchiveModels, Assembly, Issue, IssueKind, MAX_SKELETON_LEN, ModelProvider, SkeletonProblem,
};
use p4kx_extraction::Archive;
use p4kx_testkit::{TestEntry, TestModel};

const SKELETON: &str = "Data\\Objects\\Test\\rig_CHR.chr";

fn assembled(skeleton: Option<TestEntry>) -> Result<Assembly, Box<dyn std::error::Error>> {
    let mut all = entries(CHARACTER);
    all.extend(skeleton);
    let fixture = fixture(all)?;
    let archive = Archive::open(&fixture.path)?;
    assemble(&archive, "TEST_Thruster", 0)
}

fn skeleton_issues(assembly: &Assembly) -> Vec<&IssueKind> {
    assembly
        .issues()
        .iter()
        .map(Issue::kind)
        .filter(|kind| kind.code().starts_with("skeleton"))
        .collect()
}

#[test]
fn a_skeleton_that_reads_is_no_issue_even_without_its_geometry_companion() -> TestResult {
    let skeleton = TestEntry::stored(SKELETON, TestModel::two_bone_skin().cgf());

    let assembly = assembled(Some(skeleton))?;

    assert!(assembly.root().ok_or("a root")?.model().is_some());
    assert_eq!(skeleton_issues(&assembly).len(), 0);
    Ok(())
}

#[test]
fn a_missing_skeleton_is_a_note_and_the_skin_is_drawn() -> TestResult {
    let assembly = assembled(None)?;

    assert!(assembly.root().ok_or("a root")?.model().is_some());
    let [kind] = skeleton_issues(&assembly)[..] else {
        return Err("one skeleton issue".into());
    };
    assert_eq!(
        kind,
        &IssueKind::SkeletonMissing {
            path: "Objects/Test/rig_CHR.chr".to_owned()
        }
    );
    assert!(!kind.skips());
    assert!(kind.to_string().contains("rig_CHR.chr"));
    Ok(())
}

#[test]
fn a_malformed_skeleton_is_a_note_and_the_skin_is_drawn() -> TestResult {
    let skeleton = TestEntry::stored(SKELETON, b"this is no chunk file".to_vec());

    let assembly = assembled(Some(skeleton))?;

    assert!(assembly.root().ok_or("a root")?.model().is_some());
    let [IssueKind::SkeletonUnreadable { path, reason }] = skeleton_issues(&assembly)[..] else {
        return Err("one unreadable skeleton".into());
    };
    assert_eq!(path, "Objects/Test/rig_CHR.chr");
    assert_ne!(reason, "");
    Ok(())
}

#[test]
fn a_skeleton_over_the_limit_is_not_read_and_is_a_note() -> TestResult {
    let oversized = vec![0_u8; usize::try_from(MAX_SKELETON_LEN)? + 1];
    let skeleton = TestEntry::zstd(SKELETON, oversized);

    let assembly = assembled(Some(skeleton))?;

    assert!(assembly.root().ok_or("a root")?.model().is_some());
    let [IssueKind::SkeletonUnreadable { reason, .. }] = skeleton_issues(&assembly)[..] else {
        return Err("one unreadable skeleton".into());
    };
    assert!(reason.contains("more than the limit"), "{reason}");
    Ok(())
}

#[test]
fn the_provider_reports_the_problem_with_the_model() -> TestResult {
    let fixture = fixture(entries(CHARACTER))?;
    let archive = Archive::open(&fixture.path)?;

    let provided = ArchiveModels::new(&archive).model("Objects/Test/thruster.cdf", 0)?;

    assert_eq!(
        provided.skeleton,
        Some(SkeletonProblem::Missing {
            path: "Objects/Test/rig_CHR.chr".to_owned()
        })
    );
    Ok(())
}

#[test]
fn the_issues_have_a_stable_code_and_text() {
    let unreadable = IssueKind::SkeletonUnreadable {
        path: "a.chr".into(),
        reason: "too large".into(),
    };
    assert_eq!(unreadable.code(), "skeleton_unreadable");
    assert_eq!(
        unreadable.to_string(),
        "the skeleton a.chr the character definition names cannot be read: too large"
    );
}
