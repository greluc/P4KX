//! Animations in the Info tab and in Export as GLB over a synthetic Archive with a door model.

use crate::glb_exporting::{finished, start};
use crate::viewing::{Site, TestResult, model_preview, opened_at, select};
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, ModelExport, ModelExportStatus, ModelExportSummary, ModelPreview};
use p4kx_testkit::{
    TestAnimation, TestDba, TestEntry, TestModel, TestSkeleton, TestTrack, check_glb,
};

const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const TURN: [f32; 4] = [0.0, 0.0, 0.707_106_77, 0.707_106_77];

fn tip() -> u32 {
    TestSkeleton::controller_id(1)
}

fn rotating(name: &str, last_frame: u16) -> TestAnimation {
    TestAnimation::new(name, 30)
        .track(TestTrack::new(tip()).rotations(&[(0, IDENTITY), (last_frame, TURN)]))
}

pub(crate) fn site() -> Site {
    let door = TestModel::cube().with_compact_skeleton(&TestSkeleton::chain(2));
    let pack = TestDba::new()
        .animation(rotating("animations/objects/slide.caf", 60))
        .animation(
            TestAnimation::new("animations/objects/slide_back.caf", 30)
                .additive()
                .track(TestTrack::new(tip()).rotations(&[(0, IDENTITY)])),
        );
    Site {
        entries: vec![
            TestEntry::zstd("Data\\Objects\\door.cga", door.cgf()),
            TestEntry::zstd("Data\\Objects\\door.cgam", door.cgfm()),
            TestEntry::zstd("Data\\Objects\\wave.caf", rotating("wave.caf", 30).caf()),
            TestEntry::zstd("Data\\Objects\\nod.caf", rotating("nod.caf", 15).caf()),
            TestEntry::zstd("Data\\Objects\\pack.dba", pack.bytes()),
        ],
        installation: false,
        key: false,
    }
}

fn summary(model: &Model) -> Result<ModelExportSummary, String> {
    match model.model_export().map(ModelExport::status) {
        Some(ModelExportStatus::Finished(summary)) => Ok(summary.clone()),
        other => Err(format!("no summary: {other:?}")),
    }
}

#[test]
fn the_info_counts_the_animations_next_to_the_model() -> TestResult {
    let (_folder, mut harness) = opened_at(
        Language::BritishEnglish,
        crate::headless::ready(),
        site(),
        |b| b,
    )?;

    select(&mut harness, "door.cga")?;

    let info = model_preview(harness.state())
        .and_then(ModelPreview::info)
        .ok_or("no info")?;
    assert_eq!(
        info.animations,
        Some(3),
        "two files and one of the database"
    );
    Ok(())
}

#[test]
fn the_animations_are_written_when_chosen_and_the_unread_ones_are_reported() -> TestResult {
    let (folder, mut harness) = opened_at(
        Language::BritishEnglish,
        crate::headless::ready(),
        site(),
        |b| b,
    )?;
    select(&mut harness, "door.cga")?;
    harness.state_mut().begin_model_export();
    let setup = harness
        .state_mut()
        .model_export_setup_mut()
        .ok_or("no setup")?;
    assert_eq!(setup.animation_count(), 3, "offered");
    assert!(!setup.animations(), "off until chosen");
    setup.set_animations(true);
    setup.set_file(folder.path().join("door.glb"));
    assert!(harness.state_mut().submit_model_export(|| {}), "started");

    finished(&mut harness)?;

    let done = summary(harness.state())?;
    assert!(done.animations_asked(), "asked");
    assert_eq!(done.animations_written(), 3, "written");
    let [skipped] = done.unread_animations() else {
        return Err(format!("unread: {:?}", done.unread_animations()).into());
    };
    assert_eq!(
        skipped.name.as_deref(),
        Some("slide_back"),
        "the additive one"
    );
    assert_eq!(done.unread_animations_total(), 1, "total");
    assert!(
        harness.state().model_export_settings().animations,
        "remembered"
    );
    let checked = check_glb(&std::fs::read(folder.path().join("door.glb"))?)?;
    assert_eq!(checked.document.animations().count(), 3, "in the file");
    Ok(())
}

#[test]
fn no_animations_are_written_unless_chosen() -> TestResult {
    let (folder, mut harness) = opened_at(
        Language::BritishEnglish,
        crate::headless::ready(),
        site(),
        |b| b,
    )?;
    select(&mut harness, "door.cga")?;
    assert!(
        start(&mut harness, &folder.path().join("door.glb"))?,
        "started"
    );

    finished(&mut harness)?;

    let done = summary(harness.state())?;
    assert!(!done.animations_asked(), "not asked");
    assert_eq!(done.animations_written(), 0, "none");
    assert!(done.unread_animations().is_empty(), "none unread");
    let checked = check_glb(&std::fs::read(folder.path().join("door.glb"))?)?;
    assert_eq!(checked.document.animations().count(), 0, "none in the file");
    Ok(())
}

#[test]
fn the_summary_lists_the_animations_in_the_interface_language() -> TestResult {
    let (folder, mut harness) =
        opened_at(Language::German, crate::headless::ready(), site(), |b| b)?;
    select(&mut harness, "door.cga")?;
    harness.state_mut().begin_model_export();
    let setup = harness
        .state_mut()
        .model_export_setup_mut()
        .ok_or("no setup")?;
    setup.set_animations(true);
    setup.set_file(folder.path().join("door.glb"));
    assert!(harness.state_mut().submit_model_export(|| {}), "started");
    finished(&mut harness)?;

    for label in ["Animationen 3", "Nicht geschriebene Animationen (1)"] {
        assert!(harness.query_by_label_contains(label).is_some(), "{label}");
    }
    Ok(())
}
