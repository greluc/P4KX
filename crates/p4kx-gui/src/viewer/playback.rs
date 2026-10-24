// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_extraction::model::{Animation, BoneTransform};

use super::skeleton::{Pose, Sample, Skeleton};
use crate::model_info::printable;

/// The longest time one frame advances an animation, so a stalled window does not jump.
const MAX_STEP: f32 = 0.25;

/// What the user asks of the playback of a model's animations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Control {
    /// Plays or pauses the selected animation.
    Toggle,
    /// Selects the animation at this index, or none to show the bind pose.
    Select(Option<usize>),
    /// Turns repeating on or off.
    Loop(bool),
    /// Moves to this fraction, 0 to 1, of the selected animation.
    Seek(f32),
    /// Shows or hides the skeleton over the model.
    Skeleton(bool),
}

/// The skeleton overlay and the animation playback of a model: which animation plays, where it
/// stands, and the pose of the bones at that moment.
///
/// The pose is computed once per frame by [`Self::tick`] into buffers that are reused, so a
/// playing animation allocates nothing.
#[derive(Debug)]
pub struct Playback {
    skeleton: Arc<Skeleton>,
    animations: Vec<Animation>,
    names: Vec<String>,
    selected: Option<usize>,
    bindings: Vec<Option<u32>>,
    duration: f32,
    time: f32,
    playing: bool,
    switches: Switches,
    stale: bool,
    pose: Arc<Pose>,
    world: Vec<BoneTransform>,
}

#[derive(Debug, Clone, Copy)]
struct Switches {
    looping: bool,
    skeleton: bool,
}

impl Playback {
    /// Starts the playback of the model with `skeleton` and the `animations` that move it: none
    /// is selected, the skeleton is hidden and the animations repeat.
    pub(crate) fn new(skeleton: Arc<Skeleton>, animations: Vec<Animation>) -> Self {
        let bones = skeleton.bone_count();
        let names = animations
            .iter()
            .map(|animation| printable(&animation.name))
            .collect();
        Self {
            skeleton,
            animations,
            names,
            selected: None,
            bindings: Vec::new(),
            duration: 0.0,
            time: 0.0,
            playing: false,
            switches: Switches {
                looping: true,
                skeleton: false,
            },
            stale: false,
            pose: Arc::new(Pose::rest(bones)),
            world: Vec::with_capacity(bones),
        }
    }

    /// Returns the skeleton.
    #[must_use]
    pub const fn skeleton(&self) -> &Arc<Skeleton> {
        &self.skeleton
    }

    /// Returns the animations that move the skeleton, in the order they were found.
    #[must_use]
    pub fn animations(&self) -> &[Animation] {
        &self.animations
    }

    /// Returns the name of the animation at `index` as the list shows it: the name of the Archive
    /// with control characters replaced.
    #[must_use]
    pub fn name(&self, index: usize) -> Option<&str> {
        self.names.get(index).map(String::as_str)
    }

    /// Returns the index of the selected animation, or `None` while the bind pose shows.
    #[must_use]
    pub const fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Returns whether the selected animation advances.
    #[must_use]
    pub const fn is_playing(&self) -> bool {
        self.playing
    }

    /// Returns whether the selected animation starts again after its end.
    #[must_use]
    pub const fn is_looping(&self) -> bool {
        self.switches.looping
    }

    /// Returns whether the skeleton is drawn over the model.
    #[must_use]
    pub const fn shows_skeleton(&self) -> bool {
        self.switches.skeleton
    }

    /// Returns the position in the selected animation in seconds.
    #[must_use]
    pub const fn time(&self) -> f32 {
        self.time
    }

    /// Returns the length of the selected animation in seconds, or 0 while none is selected.
    #[must_use]
    pub const fn duration(&self) -> f32 {
        self.duration
    }

    /// Returns the position in the selected animation as a fraction from 0 to 1.
    #[must_use]
    pub fn fraction(&self) -> f32 {
        if self.duration > 0.0 {
            (self.time / self.duration).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Returns the pose the last [`Self::tick`] computed.
    #[must_use]
    pub const fn pose(&self) -> &Arc<Pose> {
        &self.pose
    }

    /// Applies `control`.
    pub fn apply(&mut self, control: Control) {
        match control {
            Control::Toggle => self.toggle(),
            Control::Select(animation) => self.select(animation),
            Control::Loop(looping) => self.switches.looping = looping,
            Control::Seek(fraction) => self.seek(fraction),
            Control::Skeleton(shown) => self.switches.skeleton = shown,
        }
    }

    /// Selects the animation at `index`, which then plays from its start; `None`, or an index
    /// past the list, shows the bind pose.
    pub fn select(&mut self, index: Option<usize>) {
        let animation = index.and_then(|at| self.animations.get(at).map(|found| (at, found)));
        self.selected = animation.map(|(at, _)| at);
        self.bindings = animation.map_or_else(Vec::new, |(_, found)| self.skeleton.bind(found));
        self.duration = animation.map_or(0.0, |(_, found)| found.duration_seconds());
        self.time = 0.0;
        self.playing = animation.is_some() && self.duration > 0.0;
        self.stale = true;
    }

    fn toggle(&mut self) {
        if self.playing {
            self.playing = false;
        } else if self.selected.is_some() && self.duration > 0.0 {
            if self.time >= self.duration {
                self.time = 0.0;
                self.stale = true;
            }
            self.playing = true;
        }
    }

    fn seek(&mut self, fraction: f32) {
        if self.selected.is_some() {
            let fraction = if fraction.is_finite() {
                fraction.clamp(0.0, 1.0)
            } else {
                0.0
            };
            self.time = fraction * self.duration;
            self.stale = true;
        }
    }

    /// Advances the playing animation by `seconds`, computes the pose when it changed and returns
    /// whether the animation keeps playing, so the window must draw again.
    pub fn tick(&mut self, seconds: f32) -> bool {
        if self.playing && self.duration > 0.0 {
            self.time += seconds.clamp(0.0, MAX_STEP);
            if self.switches.looping {
                if self.time > self.duration {
                    self.time %= self.duration;
                }
            } else if self.time >= self.duration {
                self.time = self.duration;
                self.playing = false;
            }
            self.stale = true;
        }
        if self.stale {
            self.refresh();
        }
        self.playing
    }

    fn refresh(&mut self) {
        let clip = self
            .selected
            .and_then(|at| self.animations.get(at))
            .map(|animation| Sample {
                animation,
                bindings: &self.bindings,
                frame: self.time * f32::from(animation.frames_per_second.max(1)),
            });
        self.skeleton
            .evaluate(clip, &mut self.world, Arc::make_mut(&mut self.pose));
        self.stale = false;
    }
}

#[cfg(test)]
mod tests;
