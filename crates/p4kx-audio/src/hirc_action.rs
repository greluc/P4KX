// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::bank_reader::SectionReader;
use crate::hirc_node::{PROP_LEN, RANGED_PROP_LEN, skip_items};
use crate::{BankError, SectionId};

const PLAY_BANK_VERSION: u32 = 154;
const PLAY_ACTION_GROUP: u16 = 0x04;
const PLAY_EVENT_ACTION_GROUP: u16 = 0x21;
const TRIGGER_ACTION_GROUP: u16 = 0x1B;

/// The soundbank a Play action names for its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayBank {
    /// The ID of the soundbank, the hash of its name; 0 for none.
    pub bank_id: u32,
    /// The type of the soundbank reference.
    pub bank_type: u32,
}

/// The action of an Action object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionTarget {
    /// The action type, e.g. `0x0403` for Play.
    pub action_type: u16,
    /// The ID of the object the action is aimed at.
    pub target_id: u32,
    /// The soundbank a Play action names, read in bank version 154 only.
    pub bank: Option<PlayBank>,
}

impl ActionTarget {
    /// Returns whether the action plays its target, action types `0x04xx`.
    #[must_use]
    pub const fn is_play(&self) -> bool {
        self.action_type >> 8 == PLAY_ACTION_GROUP
    }

    /// Returns whether the action posts the Event it is aimed at, action types `0x21xx`.
    #[must_use]
    pub const fn is_play_event(&self) -> bool {
        self.action_type >> 8 == PLAY_EVENT_ACTION_GROUP
    }

    /// Returns whether the action fires the trigger it is aimed at, action types `0x1Bxx`.
    #[must_use]
    pub const fn is_trigger(&self) -> bool {
        self.action_type >> 8 == TRIGGER_ACTION_GROUP
    }
}

/// Decodes an Action object after its ID: the type and the target and, for a Play action of bank
/// version 154, the soundbank it names, which must end the body.
pub(crate) fn action(
    version: u32,
    fields: &mut SectionReader<'_>,
) -> Result<ActionTarget, BankError> {
    let mut action = ActionTarget {
        action_type: fields.u16()?,
        target_id: fields.u32()?,
        bank: None,
    };
    if version == PLAY_BANK_VERSION && action.is_play() {
        fields.u8()?;
        let props = fields.u8()?;
        skip_items(fields, u32::from(props), PROP_LEN)?;
        let ranged = fields.u8()?;
        skip_items(fields, u32::from(ranged), RANGED_PROP_LEN)?;
        fields.u8()?;
        action.bank = Some(PlayBank {
            bank_id: fields.u32()?,
            bank_type: fields.u32()?,
        });
        if fields.remaining() > 0 {
            return Err(BankError::invalid(
                SectionId::HIRC,
                "bytes follow the fields of a play action",
            ));
        }
    }
    Ok(action)
}
