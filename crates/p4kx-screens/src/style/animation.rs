// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Animations a style entry starts, read at the state they settle in.

use p4kx_dataforge::StructValue;

use super::entry::{Modifiers, modifiers};
use crate::reader::{as_struct, child, flag, float, items, text};

/// The type name of the animation modifier, spelt as the game spells it.
pub(super) const ANIMATION: &str = "BuidlingBlocks_FieldModifierAnimation";

/// Returns the modifiers a keyframed animation leaves in force once it has run, `None` when it
/// leaves none: it loops, it does not hold its last frame (`fillMode` neither `Forward` nor
/// `Both`), it alternates, it has no embedded keyframes, or its last frame is not at the end of
/// the timeline (percent 1; percent 0 for a reverse animation). Delay and iteration count are
/// ignored.
///
/// A forward animation settles on its last keyframe, a reverse one on its first.
pub(super) fn settled(modifier: &StructValue<'_>) -> Option<Modifiers> {
    let animation = child(modifier, "value")?;
    if flag(animation, "loopIndefinitely").unwrap_or(false)
        || !matches!(text(animation, "fillMode"), Some("Forward" | "Both"))
    {
        return None;
    }
    let reverse = match text(animation, "direction") {
        Some("Forward") => false,
        Some("Reverse") => true,
        _ => return None,
    };
    let frames = items(child(animation, "animationTimeline")?, "keyframes")
        .iter()
        .filter_map(as_struct)
        .filter_map(|frame| Some((float(frame, "percent")?, frame)));
    let (percent, frame) = if reverse {
        frames.min_by(|left, right| left.0.total_cmp(&right.0))
    } else {
        frames.max_by(|left, right| left.0.total_cmp(&right.0))
    }?;
    let at_end = if reverse {
        percent <= 0.0
    } else {
        percent >= 1.0
    };
    if !at_end {
        return None;
    }
    let found = modifiers(
        items(frame, "modifiers")
            .iter()
            .filter_map(as_struct)
            .filter_map(|data| child(data, "modifier")),
    );
    (found != Modifiers::default()).then_some(found)
}
