// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::mpsc::{self, Sender};

use p4kx_extraction::CancelToken;
use p4kx_render::Scene;

use super::{Animated, Event, Loaded, ModelPreview, State};
use crate::viewer::fixtures::{skin_model, swing};
use crate::viewer::preview_animations::can_animate;
use crate::{AnimationCount, ModelInfo};

fn read_while_counting() -> (ModelPreview, Sender<Event>) {
    let (sender, events) = mpsc::channel();
    let mut preview = ModelPreview {
        stop: CancelToken::new(),
        events: Some(events),
        state: State::Loading,
        info: None,
        camera: None,
        playback: None,
    };
    let model = skin_model();
    let info = ModelInfo::of(&model).with_animations(AnimationCount::Reading);
    let scene = Scene::from_model(&model);
    assert!(
        sender
            .send(Event::Read(Box::new(Loaded { info, scene })))
            .is_ok(),
        "sent"
    );
    assert!(preview.poll(), "the model arrived");
    (preview, sender)
}

#[test]
fn the_model_is_ready_while_the_animations_are_still_counted() {
    let (preview, _sender) = read_while_counting();

    assert!(!preview.is_loading(), "the model is there");
    assert!(preview.is_reading_animations(), "the count is not");
    assert!(preview.scene().is_some(), "drawable");
}

#[test]
fn the_second_event_fills_the_count_and_the_playback() {
    let (mut preview, sender) = read_while_counting();
    let animated = Animated {
        count: AnimationCount::Counted(1),
        movers: vec![swing("swing", 30)],
    };
    assert!(sender.send(Event::Animations(animated)).is_ok(), "sent");

    assert!(preview.poll(), "the count arrived");

    assert!(!preview.is_reading_animations(), "counted");
    assert_eq!(
        preview.info().map(|info| info.animations),
        Some(AnimationCount::Counted(1)),
        "the count"
    );
    assert_eq!(
        preview
            .playback()
            .map(|playback| playback.animations().len()),
        Some(1),
        "the playback lists it"
    );
}

#[test]
fn a_reader_that_ends_while_counting_leaves_the_count_unread() {
    let (mut preview, sender) = read_while_counting();
    drop(sender);

    assert!(preview.poll(), "the end was noticed");

    assert_eq!(
        preview.info().map(|info| info.animations),
        Some(AnimationCount::Unread),
        "not read"
    );
}

#[test]
fn a_skinned_model_can_animate_whatever_the_viewer_makes_of_it() {
    assert!(can_animate(&skin_model()), "bones and a bone map");
}
