// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{AnimationChoice, AnimationSet, Archive, ModelInfo, ModelSource, PathKeys};
use p4kx_testkit::{TestAnimation, TestDba, TestEntry, TestModel, TestSkeleton, TestTrack};
use serde_json::{Value, json};

use crate::models::all_pages;
use crate::support::{Install, TestResult};

const SKIN: &str = "Data/Objects/Test/skin.skin";
const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const QUARTER_TURN_Z: [f32; 4] = [0.0, 0.0, 0.707_106_77, 0.707_106_77];

fn rotating(name: &str, last_frame: u16) -> TestAnimation {
    let tip = TestSkeleton::controller_id(1);
    TestAnimation::new(name, 30)
        .track(TestTrack::new(tip).rotations(&[(0, IDENTITY), (last_frame, QUARTER_TURN_Z)]))
}

fn install() -> TestResult<Install> {
    let skin = TestModel::two_bone_skin();
    let tip = TestSkeleton::controller_id(1);
    let pack = TestDba::new()
        .animation(rotating("animations/objects/test/slide.caf", 60))
        .animation(
            TestAnimation::new("animations/objects/test/slide_back.caf", 30)
                .additive()
                .track(TestTrack::new(tip).rotations(&[(0, IDENTITY)])),
        );
    Install::new(vec![
        TestEntry::zstd(SKIN, skin.cgf()),
        TestEntry::zstd("Data/Objects/Test/skin.skinm", skin.cgfm()),
        TestEntry::zstd(
            "Data/Animations/Objects/Test/wave.caf",
            rotating("wave.caf", 30).caf(),
        ),
        TestEntry::zstd("Data/Animations/Objects/Test/pack.dba", pack.bytes()),
    ])
}

#[test]
fn animations_come_from_caf_files_and_databases_next_to_the_model() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let archive = Archive::open(install.archive())?;
    let source = ModelSource::from_archive(&archive, SKIN)?;
    let keys = PathKeys::new(&archive);
    let animations = AnimationSet::from_archive(
        &keys.index(&archive),
        source.name(),
        &AnimationChoice::All,
        &[],
    )?;
    let expected = serde_json::to_value(ModelInfo::of(&source, &animations))?;
    let read = expected["animations"].as_array().ok_or("animations")?;
    let unread = expected["unread_animations"]
        .as_array()
        .ok_or("unread_animations")?;
    assert_eq!(read.len(), 2, "wave and the database's slide");
    assert_eq!(unread.len(), 1, "the additive animation is skipped");

    for section in ["animations", "unread_animations"] {
        let items = all_pages(
            &mut server,
            "list_model_items",
            &json!({ "path": SKIN, "section": section, "limit": 1 }),
            "items",
        )?;
        assert_eq!(Value::Array(items), expected[section], "section {section}");
    }
    let info = server.ok("get_model_info", &json!({ "path": SKIN }))?;
    assert_eq!(info["counts"]["animations"], read.len(), "animations");
    assert_eq!(
        info["counts"]["unread_animations"],
        unread.len(),
        "unread animations"
    );
    assert_eq!(
        info["counts"]["bones"],
        expected["bones"].as_array().ok_or("bones")?.len(),
        "bones"
    );
    Ok(())
}
