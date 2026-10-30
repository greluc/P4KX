// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::Transform;

use super::Hardpoint;
use super::bounds::WorldBox;
use crate::{EntityContainer, InteriorPartKind};

/// The file extension of animated geometry in the engine; a static model is a `.cgf`.
const ANIMATED_EXTENSION: &str = "cga";

pub(super) fn is_animated(model_path: Option<&str>) -> bool {
    model_path
        .and_then(|path| path.rsplit_once('.'))
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case(ANIMATED_EXTENSION))
}

/// A part of a container with animated geometry, such as a door, a ramp or a drawer, and where it
/// ended up compared with its container's room.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimatedPart {
    pub(crate) name: String,
    pub(crate) model_path: Option<String>,
    pub(crate) position: [f32; 3],
    pub(crate) distance: Option<f32>,
}

impl AnimatedPart {
    /// Returns the part's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the path of the part's model as the `.soc` file spells it.
    #[must_use]
    pub fn model_path(&self) -> Option<&str> {
        self.model_path.as_deref()
    }

    /// Returns the world position of the part's origin, in metres.
    #[must_use]
    pub const fn position(&self) -> [f32; 3] {
        self.position
    }

    /// Returns the distance in metres from the part's origin to the room box of its container,
    /// 0 inside; `None` when the container has no room box or the position is not finite.
    #[must_use]
    pub const fn distance(&self) -> Option<f32> {
        self.distance
    }
}

/// One container of an Interior measured: the box of its room and which hardpoints lie in it.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerFit {
    pub(crate) file: String,
    pub(crate) room: Option<WorldBox>,
    pub(crate) inside: Vec<usize>,
    pub(crate) outside: Vec<usize>,
    pub(crate) animated: Vec<AnimatedPart>,
}

impl ContainerFit {
    /// Measures `container`, whose owner part sits at `owner` in the world: the world box of its
    /// static model parts, the `hardpoints` inside and outside it and its animated parts.
    pub(super) fn measure(
        owner: &Transform,
        container: &EntityContainer,
        hardpoints: &[Hardpoint],
    ) -> Self {
        let base = owner.then(container.placement());
        let mut room = None;
        let mut animated = Vec::new();
        for part in container.interior().parts() {
            let (InteriorPartKind::Model | InteriorPartKind::Prop) = part.kind() else {
                continue;
            };
            let world = base.then(part.world());
            if is_animated(part.model_path()) {
                animated.push(AnimatedPart {
                    name: part.name().to_owned(),
                    model_path: part.model_path().map(str::to_owned),
                    position: world.translation(),
                    distance: None,
                });
            } else if let Some(model) = part.model() {
                WorldBox::grow_with_model(&mut room, model, &world);
            }
        }
        for part in &mut animated {
            part.distance = room.and_then(|room| room.distance(part.position));
        }
        let (mut inside, mut outside) = (Vec::new(), Vec::new());
        for (index, hardpoint) in hardpoints.iter().enumerate() {
            if room.is_some_and(|room| room.contains(hardpoint.position)) {
                inside.push(index);
            } else {
                outside.push(index);
            }
        }
        Self {
            file: container.reference().file().to_owned(),
            room,
            inside,
            outside,
            animated,
        }
    }

    /// Returns the container file as the Record spells it.
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }

    /// Returns the world box around the container's static models, or `None` when it has none
    /// that could be read.
    #[must_use]
    pub const fn room(&self) -> Option<&WorldBox> {
        self.room.as_ref()
    }

    /// Returns the indices into [`InteriorFit::hardpoints`] of the hardpoints inside the room box.
    #[must_use]
    pub fn inside(&self) -> &[usize] {
        &self.inside
    }

    /// Returns the indices into [`InteriorFit::hardpoints`] of the hardpoints outside the room box,
    /// every one when the container has no room box.
    #[must_use]
    pub fn outside(&self) -> &[usize] {
        &self.outside
    }

    /// Returns the container's parts with animated geometry (`.cga` models), left out of the room
    /// box.
    #[must_use]
    pub fn animated(&self) -> &[AnimatedPart] {
        &self.animated
    }
}

/// A hardpoint and the room box nearest to it.
#[derive(Debug, Clone, PartialEq)]
pub struct HardpointFit {
    pub(crate) hardpoint: Hardpoint,
    pub(crate) container: Option<usize>,
    pub(crate) distance: Option<f32>,
}

impl HardpointFit {
    /// Measures `hardpoint` against the room boxes of `containers`; the nearest wins, the first
    /// of equals.
    pub(super) fn measure(hardpoint: Hardpoint, containers: &[ContainerFit]) -> Self {
        let nearest = containers
            .iter()
            .enumerate()
            .filter_map(|(index, container)| {
                let distance = container.room?.distance(hardpoint.position)?;
                Some((index, distance))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b));
        Self {
            hardpoint,
            container: nearest.map(|(index, _)| index),
            distance: nearest.map(|(_, distance)| distance),
        }
    }

    /// Returns the hardpoint.
    #[must_use]
    pub const fn hardpoint(&self) -> &Hardpoint {
        &self.hardpoint
    }

    /// Returns the index into [`InteriorFit::containers`] of the container whose room box is
    /// nearest to the hardpoint, or `None` when no container has a box or the position is not
    /// finite.
    #[must_use]
    pub const fn container(&self) -> Option<usize> {
        self.container
    }

    /// Returns the distance in metres from the hardpoint to that box, 0 inside.
    #[must_use]
    pub const fn distance(&self) -> Option<f32> {
        self.distance
    }

    /// Returns whether the hardpoint is at most `tolerance` metres from a room box; a hardpoint
    /// with no box to measure against is not.
    #[must_use]
    pub fn is_within(&self, tolerance: f32) -> bool {
        self.distance.is_some_and(|distance| distance <= tolerance)
    }
}
