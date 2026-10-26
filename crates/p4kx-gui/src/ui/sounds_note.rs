// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{Playable, SoundsText};

/// Returns the note that says why a sound that is `playable` has no player, or `None` when it
/// has one; a sound that should play but has no player has no Entry in the Archive to read.
pub(super) const fn player_note(playable: Playable, has_player: bool) -> Option<SoundsText> {
    match playable {
        Playable::Yes if has_player => None,
        Playable::Yes => Some(SoundsText::NoteNoEntry),
        Playable::PrefetchOnly => Some(SoundsText::NotePrefetch),
        Playable::Plugin => Some(SoundsText::NotePlugin),
        Playable::External => Some(SoundsText::NoteExternal),
        Playable::Midi => Some(SoundsText::NoteMidi),
        Playable::Missing => Some(SoundsText::NoteMissing),
    }
}

#[cfg(test)]
mod tests {
    use super::{Playable, SoundsText, player_note};

    #[test]
    fn a_playable_sound_without_a_player_names_the_missing_entry() {
        assert_eq!(
            player_note(Playable::Yes, false),
            Some(SoundsText::NoteNoEntry)
        );
    }

    #[test]
    fn a_playable_sound_with_a_player_has_no_note() {
        assert_eq!(player_note(Playable::Yes, true), None);
    }

    #[test]
    fn a_sound_that_cannot_play_keeps_its_own_note_whatever_the_player() {
        assert_eq!(
            player_note(Playable::Plugin, true),
            Some(SoundsText::NotePlugin)
        );
        assert_eq!(
            player_note(Playable::Missing, false),
            Some(SoundsText::NoteMissing)
        );
    }
}
