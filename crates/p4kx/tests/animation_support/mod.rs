//! The Archive and the helpers of the animation tests of `p4kx model`.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, CheckedGlb, TestAnimation, TestDba, TestEntry, TestModel, TestSkeleton,
    TestTrack,
};

pub(crate) const SKIN: &str = "Data\\Objects\\Test\\skin.skin";
pub(crate) const SKIN_GEOMETRY: &str = "Data\\Objects\\Test\\skin.skinm";
pub(crate) const DOOR: &str = "Data\\Objects\\Test\\door.cga";
pub(crate) const DOOR_GEOMETRY: &str = "Data\\Objects\\Test\\door.cgam";
pub(crate) const WAVE: &str = "Data\\Animations\\Objects\\Test\\wave.caf";
pub(crate) const NOD: &str = "Data\\Animations\\Objects\\Test\\nod.caf";
pub(crate) const PACK: &str = "Data\\Animations\\Objects\\Test\\pack.dba";
pub(crate) const FAR: &str = "Data\\Animations\\Elsewhere\\far.caf";
pub(crate) const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub(crate) const QUARTER_TURN_Z: [f32; 4] = [0.0, 0.0, 0.707_106_77, 0.707_106_77];

pub(crate) fn tip() -> u32 {
    TestSkeleton::controller_id(1)
}

pub(crate) fn rotating(name: &str, controller: u32, last_frame: u16) -> TestAnimation {
    TestAnimation::new(name, 30)
        .track(TestTrack::new(controller).rotations(&[(0, IDENTITY), (last_frame, QUARTER_TURN_Z)]))
}

pub(crate) fn write_archive(folder: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    let skin = TestModel::two_bone_skin();
    let door = TestModel::cube().with_compact_skeleton(&TestSkeleton::chain(2));
    let pack = TestDba::new()
        .animation(rotating("animations/objects/test/slide.caf", tip(), 60))
        .animation(
            TestAnimation::new("animations/objects/test/slide_back.caf", 30)
                .additive()
                .track(TestTrack::new(tip()).rotations(&[(0, IDENTITY)]))
                .track(TestTrack::new(0xBAD0_0001).rotations(&[(0, IDENTITY)])),
        );
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(SKIN, skin.cgf()))
        .entry(TestEntry::zstd(SKIN_GEOMETRY, skin.cgfm()))
        .entry(TestEntry::zstd(DOOR, door.cgf()))
        .entry(TestEntry::zstd(DOOR_GEOMETRY, door.cgfm()))
        .entry(TestEntry::zstd(WAVE, rotating("wave.caf", tip(), 30).caf()))
        .entry(TestEntry::zstd(NOD, rotating("nod.caf", tip(), 15).caf()))
        .entry(TestEntry::zstd(PACK, pack.bytes()))
        .entry(TestEntry::zstd(FAR, rotating("far.caf", tip(), 90).caf()))
        .write_to(&path)?;
    Ok(path)
}

pub(crate) fn p4kx() -> Result<assert_cmd::Command, Box<dyn Error>> {
    Ok(p4kx_testkit::p4kx()?)
}

pub(crate) fn glb_of(
    archive: &Path,
    out: &Path,
    extra: &[&str],
) -> Result<std::process::Output, Box<dyn Error>> {
    Ok(p4kx()?
        .args(["model", "glb"])
        .arg(archive)
        .arg(SKIN)
        .arg("--out")
        .arg(out)
        .args(extra)
        .output()?)
}

pub(crate) fn animation_names(glb: &CheckedGlb) -> Vec<String> {
    glb.document
        .animations()
        .map(|animation| animation.name().unwrap_or_default().to_owned())
        .collect()
}
