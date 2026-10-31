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

    /// Returns the distance in metres from the part's origin to the nearest static model's box of
    /// its container, 0 inside; `None` when the container has no readable static model or the
    /// position is not finite.
    #[must_use]
    pub const fn distance(&self) -> Option<f32> {
        self.distance
    }
}

/// One container of an Interior measured: the boxes of its static models and which hardpoints lie
/// in them.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerFit {
    pub(crate) file: String,
    pub(crate) owner: usize,
    pub(crate) room: Option<WorldBox>,
    pub(crate) models: Vec<WorldBox>,
    pub(crate) rooms: Vec<WorldBox>,
    pub(crate) inside: Vec<usize>,
    pub(crate) outside: Vec<usize>,
    pub(crate) animated: Vec<AnimatedPart>,
    pub(crate) unreadable: usize,
}

/// The share of the largest static model's box volume a model needs to count as a room model.
///
/// Only the hull check ([`ContainerFit::overshoot`]) uses room models: dropping a small model
/// can only weaken it, never raise a false alarm. On the 15 real ships measured on LIVE the
/// smallest room model has a ratio of 0.251 and the largest dropped model 0.248, so a quarter is
/// a cut inside a continuum, not a gap; the Cutlass cockpit model has 0.31. Hardpoints are
/// measured against every static model instead: real cockpit and bridge seats lie in models
/// with ratios down to 0.003.
pub const ROOM_VOLUME_SHARE: f32 = 0.25;

/// Returns the boxes of `models` that are room models: at least [`ROOM_VOLUME_SHARE`] of the
/// largest volume.
fn room_models(models: &[WorldBox]) -> Vec<WorldBox> {
    let largest = models.iter().map(WorldBox::volume).fold(0.0, f32::max);
    models
        .iter()
        .filter(|model| model.volume() >= largest * ROOM_VOLUME_SHARE)
        .copied()
        .collect()
}

/// Returns the distance in metres from `point` to the nearest of `boxes`, 0 inside one, or
/// `None` when there is none or `point` is not finite.
pub(super) fn distance_to_boxes(boxes: &[WorldBox], point: [f32; 3]) -> Option<f32> {
    boxes
        .iter()
        .filter_map(|bounds| bounds.distance(point))
        .min_by(f32::total_cmp)
}

/// Splits the indices of `hardpoints` into those inside one of `boxes` and the rest.
pub(super) fn classify(boxes: &[WorldBox], hardpoints: &[Hardpoint]) -> (Vec<usize>, Vec<usize>) {
    let (mut inside, mut outside) = (Vec::new(), Vec::new());
    for (index, hardpoint) in hardpoints.iter().enumerate() {
        if boxes
            .iter()
            .any(|bounds| bounds.contains(hardpoint.position))
        {
            inside.push(index);
        } else {
            outside.push(index);
        }
    }
    (inside, outside)
}

impl ContainerFit {
    /// Measures `container`, whose owner part sits at `owner` in the world: the world box of the
    /// static models its `.soc` files place (props such as doors, lockers and cabinets do not
    /// bound the room), the `hardpoints` inside and outside it and its animated parts.
    pub(super) fn measure(
        owner: &Transform,
        container: &EntityContainer,
        hardpoints: &[Hardpoint],
    ) -> Self {
        let base = owner.then(container.placement());
        let mut room = None;
        let mut models = Vec::new();
        let mut animated = Vec::new();
        let mut unreadable = 0;
        for part in container.interior().parts() {
            let kind = part.kind();
            if !matches!(kind, InteriorPartKind::Model | InteriorPartKind::Prop) {
                continue;
            }
            let world = base.then(part.world());
            if is_animated(part.model_path()) {
                animated.push(AnimatedPart {
                    name: part.name().to_owned(),
                    model_path: part.model_path().map(str::to_owned),
                    position: world.translation(),
                    distance: None,
                });
            } else if kind == InteriorPartKind::Model {
                match part.model() {
                    Some(model) => {
                        let mut own = None;
                        WorldBox::grow_with_model(&mut own, model, &world);
                        if let Some(own) = own {
                            WorldBox::grow_with_box(&mut room, &own);
                            models.push(own);
                        }
                    }
                    None => unreadable += 1,
                }
            }
        }
        let rooms = room_models(&models);
        for part in &mut animated {
            part.distance = distance_to_boxes(&models, part.position);
        }
        let (inside, outside) = classify(&models, hardpoints);
        Self {
            file: container.reference().file().to_owned(),
            owner: container.part(),
            room,
            models,
            rooms,
            inside,
            outside,
            animated,
            unreadable,
        }
    }

    /// Returns whether `point` is at most `tolerance` metres from the box of a single static
    /// model, not from the union of all of them; a container without a readable static model
    /// holds nothing.
    #[must_use]
    pub fn holds(&self, point: [f32; 3], tolerance: f32) -> bool {
        distance_to_boxes(&self.models, point).is_some_and(|distance| distance <= tolerance)
    }

    /// Returns the boxes of the container's room models: its static models whose volume is at
    /// least [`ROOM_VOLUME_SHARE`] of the largest one's. A socpak may place the model of a
    /// neighbouring hull section next to its own, so the union box [`Self::room`] can be much
    /// longer than the room the container stands for. A container whose static models are all
    /// unreadable has none.
    #[must_use]
    pub fn rooms(&self) -> &[WorldBox] {
        &self.rooms
    }

    /// Returns the boxes of all the container's readable static models, one per model, which
    /// hardpoints and animated parts are measured against; [`Self::rooms`] is the larger part.
    #[must_use]
    pub fn models(&self) -> &[WorldBox] {
        &self.models
    }

    /// Returns the number of static model parts whose model could not be read; they are missing
    /// from the room box.
    #[must_use]
    pub const fn unreadable_models(&self) -> usize {
        self.unreadable
    }

    /// Returns how far in metres the room models protrude beyond `hull` on their worst axis, 0
    /// when all lie inside; `None` when the container has no room model.
    #[must_use]
    pub fn overshoot(&self, hull: &WorldBox) -> Option<f32> {
        self.rooms
            .iter()
            .map(|room| room.overshoot(hull))
            .max_by(f32::total_cmp)
    }

    /// Returns the index in the Assembly's parts of the Entity that names the container: 0 for
    /// the root Entity, otherwise an attached one such as a docked ship.
    #[must_use]
    pub const fn owner(&self) -> usize {
        self.owner
    }

    /// Returns the container file as the Record spells it.
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }

    /// Returns the union box around the container's static models, or `None` when it has none
    /// that could be read. Hardpoints and animated parts are measured against
    /// [`Self::models`], not against this box.
    #[must_use]
    pub const fn room(&self) -> Option<&WorldBox> {
        self.room.as_ref()
    }

    /// Returns the indices into [`InteriorFit::hardpoints`] of the hardpoints inside the box of a
    /// static model.
    #[must_use]
    pub fn inside(&self) -> &[usize] {
        &self.inside
    }

    /// Returns the indices into [`InteriorFit::hardpoints`] of the hardpoints outside the box of
    /// every static model, every one when the container has none.
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
    /// Measures `hardpoint` against the static-model boxes of `containers`; the nearest wins, the
    /// first of equals.
    pub(super) fn measure(hardpoint: Hardpoint, containers: &[ContainerFit]) -> Self {
        let nearest = containers
            .iter()
            .enumerate()
            .filter_map(|(index, container)| {
                let distance = distance_to_boxes(&container.models, hardpoint.position)?;
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

    /// Returns the index into [`InteriorFit::containers`] of the container whose static-model box
    /// is nearest to the hardpoint, or `None` when no container has a box or the position is not
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

    /// Returns whether the hardpoint is at most `tolerance` metres from a static-model box; a
    /// hardpoint with no box to measure against is not.
    #[must_use]
    pub fn is_within(&self, tolerance: f32) -> bool {
        self.distance.is_some_and(|distance| distance <= tolerance)
    }

    /// Returns whether the hardpoint lies inside the box of a static model, on its surface
    /// included.
    #[must_use]
    pub fn is_inside(&self) -> bool {
        self.is_within(0.0)
    }
}
