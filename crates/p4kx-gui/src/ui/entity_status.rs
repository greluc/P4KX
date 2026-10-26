// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::widgets::chip::Tone;
use crate::{EntityText, RowChange};

pub(super) const fn own_status(
    change: RowChange,
    docked: bool,
    empty_leaf: bool,
) -> Option<(EntityText, Tone)> {
    match change {
        RowChange::Excluded => Some((EntityText::StatusExcluded, Tone::Warning)),
        RowChange::Replaced => Some((EntityText::StatusReplaced, Tone::Info)),
        RowChange::Kept if docked => Some((EntityText::StatusDocked, Tone::Muted)),
        RowChange::Kept if empty_leaf => Some((EntityText::StatusEmpty, Tone::Muted)),
        RowChange::Kept => None,
    }
}

/// Returns the chips of a row in the order they are laid out, the hidden chip before the
/// chip of the row's own state; an unresolved row has that chip alone.
pub(super) fn status_kinds(
    unresolved: bool,
    hidden: bool,
    change: RowChange,
    own: Option<(EntityText, Tone)>,
) -> Vec<(EntityText, Tone)> {
    if unresolved {
        return vec![(EntityText::StatusUnresolved, Tone::Danger)];
    }
    let hidden = (hidden && change != RowChange::Excluded)
        .then_some((EntityText::StatusHidden, Tone::Muted));
    hidden.into_iter().chain(own).collect()
}

#[cfg(test)]
mod tests {
    use super::{EntityText, RowChange, Tone, own_status, status_kinds};

    fn kinds(unresolved: bool, hidden: bool, change: RowChange) -> Vec<EntityText> {
        let own = own_status(change, false, false);
        status_kinds(unresolved, hidden, change, own)
            .into_iter()
            .map(|(text, _)| text)
            .collect()
    }

    #[test]
    fn a_replaced_hidden_port_has_both_chips() {
        assert_eq!(
            kinds(false, true, RowChange::Replaced),
            [EntityText::StatusHidden, EntityText::StatusReplaced]
        );
    }

    #[test]
    fn a_left_out_port_never_shows_hidden() {
        assert_eq!(
            kinds(false, true, RowChange::Excluded),
            [EntityText::StatusExcluded]
        );
    }

    #[test]
    fn an_unresolved_port_has_that_chip_alone() {
        assert_eq!(
            kinds(true, true, RowChange::Replaced),
            [EntityText::StatusUnresolved]
        );
    }

    #[test]
    fn a_kept_visible_port_has_none_and_a_docked_one_has_its_chip() {
        assert_eq!(kinds(false, false, RowChange::Kept).len(), 0);
        assert_eq!(
            own_status(RowChange::Kept, true, false),
            Some((EntityText::StatusDocked, Tone::Muted))
        );
    }
}
