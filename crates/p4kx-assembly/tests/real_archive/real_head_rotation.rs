// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The rotations of the facial joints against motions whose direction is known: looking up turns
//! the pupil up, a blink moves the upper eyelid down, the jaw opening moves the chin down and
//! back. Each is evaluated with the convention of the rig logic and with the three others it could
//! have, which shows what the data settles (ADR-0098).

#![expect(
    clippy::indexing_slicing,
    clippy::suboptimal_flops,
    reason = "the indices come from the same clip and skeleton, and the quaternion formulas read as written"
)]

use std::error::Error;
use std::io::Read;

use p4kx_assembly::character::Chf;
use p4kx_assembly::{ArchiveModels, Armour, AssemblyOptions, Expressions, Head};
use p4kx_character::RigLogic;
use p4kx_chunks::{ChunkFile, ivo};
use p4kx_extraction::{AnimationChoice, AnimationSet, Archive};
use p4kx_gltf::SampledAnimation;
use p4kx_model::{Animation, Bone, multiply_quaternions};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const CLIP: &str = "facerom_t0_face";

/// The ways the delta of a joint could be applied to its bind rotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Convention {
    BindTimesDelta,
    DeltaTimesBind,
    BindTimesInverse,
    InverseTimesBind,
}

const CONVENTIONS: [Convention; 4] = [
    Convention::BindTimesDelta,
    Convention::DeltaTimesBind,
    Convention::BindTimesInverse,
    Convention::InverseTimesBind,
];

struct Fixture {
    logic: RigLogic,
    armour: Armour,
    set: AnimationSet,
}

fn read(archive: &Archive, ends_with: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let entry = archive
        .entries()
        .iter()
        .find(|entry| {
            entry
                .path()
                .to_ascii_lowercase()
                .replace('\\', "/")
                .ends_with(ends_with)
        })
        .ok_or_else(|| format!("the Archive ships no {ends_with}"))?;
    let mut bytes = Vec::new();
    archive.open_entry(entry)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn fixture(archive: &Archive, models: &ArchiveModels<'_>) -> Result<Fixture, Box<dyn Error>> {
    let skin = read(archive, "protos_human_male_face_t1_pu_head.skin")?;
    let chunks = ChunkFile::parse(skin.as_slice())?;
    let (index, _) = chunks.chunks_of(ivo::RIG_LOGIC).next().ok_or("RigLogic")?;
    let logic = RigLogic::read(chunks.chunk_data(index).ok_or("RigLogic data")?)?;
    let chf = Chf::read(&read(archive, "masculinedefault.chf")?)?;
    let head = Head::from_character(&chf.character)?.with_expressions(&Expressions::None);
    let variant = head.variant();
    let armour = Armour::build(variant, head.pieces(), models, &AssemblyOptions::default())?;
    let set = models.animations(
        &variant.animation_folders(),
        &AnimationChoice::Named(vec![CLIP.to_owned()]),
        &[],
    )?;
    Ok(Fixture { logic, armour, set })
}

fn conjugate([x, y, z, w]: [f32; 4]) -> [f32; 4] {
    [-x, -y, -z, w]
}

fn rotate(q: [f32; 4], [x, y, z]: [f32; 3]) -> [f32; 3] {
    let turned = multiply_quaternions(multiply_quaternions(q, [x, y, z, 0.0]), conjugate(q));
    [turned[0], turned[1], turned[2]]
}

fn recombine(convention: Convention, bind: [f32; 4], sampled: [f32; 4]) -> [f32; 4] {
    let delta = multiply_quaternions(conjugate(bind), sampled);
    match convention {
        Convention::BindTimesDelta => multiply_quaternions(bind, delta),
        Convention::DeltaTimesBind => multiply_quaternions(delta, bind),
        Convention::BindTimesInverse => multiply_quaternions(bind, conjugate(delta)),
        Convention::InverseTimesBind => multiply_quaternions(conjugate(delta), bind),
    }
}

fn angle(a: [f32; 4], b: [f32; 4]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    (2.0 * dot.abs().min(1.0).acos()).to_degrees()
}

struct Pose<'a> {
    bones: &'a [Bone],
    sampled: &'a SampledAnimation,
    at: usize,
}

impl Pose<'_> {
    fn local(&self, bone: usize, convention: Convention) -> ([f32; 3], [f32; 4]) {
        let rest = &self.bones[bone];
        self.sampled
            .bones
            .iter()
            .find(|moved| moved.bone == bone)
            .map_or(
                (rest.relative.translation, rest.relative.rotation),
                |moved| {
                    (
                        moved.translations[self.at],
                        recombine(convention, rest.relative.rotation, moved.rotations[self.at]),
                    )
                },
            )
    }

    fn world(&self, bone: usize, convention: Convention) -> [f32; 3] {
        let mut chain = vec![bone];
        while let Some(parent) = chain.last().and_then(|&link| self.bones[link].parent) {
            chain.push(parent);
        }
        let mut position = [0.0_f32; 3];
        let mut rotation = [0.0, 0.0, 0.0, 1.0];
        for &link in chain.iter().rev() {
            let (translation, turn) = self.local(link, convention);
            for (slot, by) in position.iter_mut().zip(rotate(rotation, translation)) {
                *slot += by;
            }
            rotation = multiply_quaternions(rotation, turn);
        }
        position
    }
}

fn peak(animation: &Animation, logic: &RigLogic, control: &str) -> Option<(u16, f32)> {
    let track = animation.tracks.iter().find(|track| {
        logic
            .control_by_hash(track.controller_id)
            .and_then(|number| logic.name(number))
            .as_deref()
            == Some(control)
    })?;
    let keys = track.translations.as_ref()?;
    keys.frames
        .iter()
        .zip(&keys.values)
        .map(|(&frame, value)| (frame, value[0]))
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

fn bone_named(bones: &[Bone], name: &str) -> Result<usize, Box<dyn Error>> {
    bones
        .iter()
        .position(|bone| bone.name == name)
        .ok_or_else(|| format!("the skeleton has no bone {name}").into())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the displacements are the measurement")]
fn the_rotations_move_the_face_the_way_the_expressions_name() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let models = ArchiveModels::new(&archive);
    let Fixture { logic, armour, set } = fixture(&archive, &models)?;
    let animation = set.animations.first().ok_or("the clip")?;
    let animations = armour.animate(&set.animations, &models);
    let sampled = animations.sampled().first().ok_or("the sampled clip")?;
    let bones = armour.skeleton().bones.as_slice();
    let rate = f32::from(animation.frames_per_second);

    let probes = [
        ("CTL_expressions eyeLookUpL", "FACIAL_LOD1_L_Pupil", 1.0),
        ("CTL_expressions eyeLookDownL", "FACIAL_LOD1_L_Pupil", -1.0),
        ("CTL_expressions eyeLookUpR", "FACIAL_LOD1_R_Pupil", 1.0),
        ("CTL_expressions eyeLookDownR", "FACIAL_LOD1_R_Pupil", -1.0),
        (
            "CTL_expressions blinkL",
            "FACIAL_LOD0_L_EyelidUpperSkin1",
            -1.0,
        ),
        (
            "CTL_expressions blinkR",
            "FACIAL_LOD0_R_EyelidUpperSkin1",
            -1.0,
        ),
        ("CTL_C_jaw ty", "FACIAL_LOD1_C_Chin", -1.0),
    ];
    for (control, probe, up) in probes {
        let (frame, value) = peak(animation, &logic, control).ok_or(control)?;
        let time = f32::from(frame) / rate;
        let (at, _) = sampled
            .times
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 - time).abs().total_cmp(&(b.1 - time).abs()))
            .ok_or("a sample")?;
        assert!((sampled.times[at] - time).abs() < 0.05, "{control}");
        let pose = Pose { bones, sampled, at };
        let probe = bone_named(bones, probe)?;
        let bind = bones[probe].world.translation;
        let rises = |convention| pose.world(probe, convention)[2] - bind[2];
        println!(
            "{control} = {value:.2} at frame {frame}, {}:",
            bones[probe].name
        );
        for convention in CONVENTIONS {
            let by = pose.world(probe, convention);
            println!(
                "  {convention:?}: {:.1} {:.1} {:.1} mm",
                (by[0] - bind[0]) * 1000.0,
                (by[1] - bind[1]) * 1000.0,
                (by[2] - bind[2]) * 1000.0,
            );
        }
        assert!(rises(Convention::BindTimesDelta) * up > 0.002, "{control}");
        for refuted in [Convention::BindTimesInverse, Convention::InverseTimesBind] {
            assert!(rises(refuted) * up < 0.0, "{control}: {refuted:?}");
        }
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the angles are the measurement")]
fn the_euler_order_and_the_side_of_the_delta_change_the_poses_the_data_cannot_judge() -> TestResult
{
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let models = ArchiveModels::new(&archive);
    let Fixture { armour, set, .. } = fixture(&archive, &models)?;
    let animations = armour.animate(&set.animations, &models);
    let sampled = animations.sampled().first().ok_or("the sampled clip")?;
    let bones = armour.skeleton().bones.as_slice();
    let (mut order, mut side, mut turn) = (0.0_f32, 0.0_f32, 0.0_f32);
    for moved in &sampled.bones {
        let bind = bones[moved.bone].relative.rotation;
        for &rotation in &moved.rotations {
            let delta = multiply_quaternions(conjugate(bind), rotation);
            order = order.max(angle(delta, reversed_order(delta)));
            side = side.max(angle(
                multiply_quaternions(bind, delta),
                multiply_quaternions(delta, bind),
            ));
            turn = turn.max(angle(delta, [0.0, 0.0, 0.0, 1.0]));
        }
    }
    println!(
        "{CLIP}: largest delta {turn:.1} deg; ZYX instead of XYZ moves a joint by up to {order:.1} deg, delta*bind instead of bind*delta by up to {side:.1} deg"
    );
    assert!(order > 5.0 && side > 5.0, "{order} {side}");
    Ok(())
}

fn axis_quaternion(radians: f32, axis: usize) -> [f32; 4] {
    let (s, c) = (radians * 0.5).sin_cos();
    let mut q = [0.0, 0.0, 0.0, c];
    q[axis] = s;
    q
}

/// Returns the quaternion of the same Euler angles `x, y, z` of `q` (extracted as XYZ) applied in
/// the order Z, Y, X.
fn reversed_order([x, y, z, w]: [f32; 4]) -> [f32; 4] {
    let r02 = 2.0 * (x * z + w * y);
    let r12 = 2.0 * (y * z - w * x);
    let r22 = 1.0 - 2.0 * (x * x + y * y);
    let r01 = 2.0 * (x * y - w * z);
    let r00 = 1.0 - 2.0 * (y * y + z * z);
    let (ex, ey, ez) = (
        (-r12).atan2(r22),
        r02.clamp(-1.0, 1.0).asin(),
        (-r01).atan2(r00),
    );
    multiply_quaternions(
        multiply_quaternions(axis_quaternion(ez, 2), axis_quaternion(ey, 1)),
        axis_quaternion(ex, 0),
    )
}
