// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{InputState, Key, Modifiers, Vec2};

use crate::{Camera, Step};

const POINTS_PER_WHEEL_STEP: f32 = 40.0;
const PINCH_STEP: f32 = 0.105_360_52;

/// What the pointer and keyboard asked of the camera in one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Moves {
    pub(super) orbit: Vec2,
    pub(super) pan: Vec2,
    pub(super) wheel: f32,
    pub(super) pinch: f32,
    pub(super) orbit_step: Option<Step>,
    pub(super) pan_step: Option<Step>,
    pub(super) zoom_steps: f32,
    pub(super) reset: bool,
}

impl Default for Moves {
    fn default() -> Self {
        Self {
            orbit: Vec2::ZERO,
            pan: Vec2::ZERO,
            wheel: 0.0,
            pinch: 1.0,
            orbit_step: None,
            pan_step: None,
            zoom_steps: 0.0,
            reset: false,
        }
    }
}

/// Reads the keys of the focused view into `moves`: the arrows orbit, with Shift they pan, plus
/// and minus zoom and Home resets.
pub(super) fn read_keys(input: &mut InputState, moves: &mut Moves) {
    const ARROWS: [(Key, Step); 4] = [
        (Key::ArrowLeft, Step::Left),
        (Key::ArrowRight, Step::Right),
        (Key::ArrowUp, Step::Up),
        (Key::ArrowDown, Step::Down),
    ];
    for (key, step) in ARROWS {
        if input.consume_key(Modifiers::SHIFT, key) {
            moves.pan_step = Some(step);
        }
        if input.consume_key(Modifiers::NONE, key) {
            moves.orbit_step = Some(step);
        }
    }
    for key in [Key::Plus, Key::Equals] {
        if input.consume_key(Modifiers::NONE, key) || input.consume_key(Modifiers::SHIFT, key) {
            moves.zoom_steps += 1.0;
        }
    }
    if input.consume_key(Modifiers::NONE, Key::Minus) {
        moves.zoom_steps -= 1.0;
    }
    if input.consume_key(Modifiers::NONE, Key::Home) {
        moves.reset = true;
    }
}

/// Moves `camera` as `moves` ask, in a view `aspect` wide per height that is `height` points
/// high.
pub(super) fn apply(camera: &mut Camera, moves: &Moves, aspect: f32, height: f32) {
    if moves.reset {
        camera.reset(aspect);
        return;
    }
    if moves.orbit != Vec2::ZERO {
        camera.orbit(moves.orbit.x, moves.orbit.y);
    }
    if moves.pan != Vec2::ZERO {
        camera.pan(moves.pan.x, moves.pan.y, height);
    }
    if let Some(step) = moves.orbit_step {
        camera.step(step);
    }
    if let Some(step) = moves.pan_step {
        camera.pan_step(step, height);
    }
    let pinch = if moves.pinch > 0.0 && moves.pinch.is_finite() {
        moves.pinch.ln() / PINCH_STEP
    } else {
        0.0
    };
    camera.zoom(moves.wheel / POINTS_PER_WHEEL_STEP + pinch + moves.zoom_steps);
}

#[cfg(test)]
mod tests {
    use egui::vec2;

    use super::{Moves, apply};
    use crate::{Bounds, Camera};

    const ASPECT: f32 = 1.5;

    fn camera() -> Camera {
        let mut camera = Camera::framing(&Bounds {
            min: [-1.0; 3],
            max: [1.0; 3],
        });
        camera.fit(ASPECT);
        camera
    }

    #[test]
    fn a_drag_orbits_and_the_wheel_zooms() {
        let mut moved = camera();

        apply(
            &mut moved,
            &Moves {
                orbit: vec2(20.0, -10.0),
                wheel: 80.0,
                ..Moves::default()
            },
            ASPECT,
            300.0,
        );

        assert_ne!(moved.eye(), camera().eye(), "the eye moved");
        assert!(moved.distance() < camera().distance(), "closer");
    }

    #[test]
    fn a_pinch_out_zooms_in_and_a_pinch_in_zooms_out() {
        let mut closer = camera();
        let mut farther = camera();

        apply(
            &mut closer,
            &Moves {
                pinch: 1.5,
                ..Moves::default()
            },
            ASPECT,
            300.0,
        );
        apply(
            &mut farther,
            &Moves {
                pinch: 0.5,
                ..Moves::default()
            },
            ASPECT,
            300.0,
        );

        assert!(closer.distance() < camera().distance(), "closer");
        assert!(farther.distance() > camera().distance(), "farther");
    }

    #[test]
    fn no_movement_leaves_the_camera_unchanged() {
        let mut idle = camera();

        apply(&mut idle, &Moves::default(), ASPECT, 300.0);

        assert_eq!(idle, camera(), "unchanged");
    }

    #[test]
    fn the_reset_action_wins_over_every_other_movement() {
        let mut moved = camera();

        apply(
            &mut moved,
            &Moves {
                orbit: vec2(50.0, 50.0),
                zoom_steps: 5.0,
                reset: true,
                ..Moves::default()
            },
            ASPECT,
            300.0,
        );

        assert_eq!(moved, camera(), "back at the start");
    }

    #[test]
    fn a_degenerate_pinch_is_ignored() {
        let mut idle = camera();

        for pinch in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            apply(
                &mut idle,
                &Moves {
                    pinch,
                    ..Moves::default()
                },
                ASPECT,
                300.0,
            );
        }

        assert_eq!(idle, camera(), "unchanged");
    }
}
