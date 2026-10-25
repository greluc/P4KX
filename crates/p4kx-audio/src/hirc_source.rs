// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::bank_reader::SectionReader;
use crate::hirc::{self, MediaStorage, SoundSource};
use crate::hirc_node;
use crate::{BankError, SectionId};

const FIRST_SOUND_VERSION: u32 = 150;
const NODE_BASE_VERSION: u32 = 154;
const PLUGIN_TYPE_MASK: u32 = 0x0F;
const SOURCE_PLUGIN_TYPE: u32 = 2;

/// Decodes a Sound object after its ID: the media source, then in bank version 154 the plug-in
/// parameters of a source plug-in and the node base parameters, which must end the body.
pub(crate) fn sound(
    version: u32,
    fields: &mut SectionReader<'_>,
) -> Result<SoundSource, BankError> {
    let plugin_id = fields.u32()?;
    let storage = MediaStorage::from_byte(fields.u8()?);
    let source_id = fields.u32()?;
    if hirc::has_cache_id(version) {
        fields.u32()?;
    }
    let in_memory_size = fields.u32()?;
    let source_bits = if fields.remaining() > 0 {
        fields.u8()?
    } else {
        0
    };
    let parent_id = if version == NODE_BASE_VERSION && fields.remaining() > 0 {
        if plugin_id & PLUGIN_TYPE_MASK == SOURCE_PLUGIN_TYPE {
            let len = usize::try_from(fields.u32()?).unwrap_or(usize::MAX);
            fields.skip(len)?;
        }
        let parent_id = hirc_node::node_base(fields)?;
        if fields.remaining() > 0 {
            return Err(BankError::invalid(
                SectionId::HIRC,
                "bytes follow the fields of a sound",
            ));
        }
        Some(parent_id)
    } else {
        None
    };
    Ok(SoundSource {
        plugin_id,
        storage,
        source_id,
        in_memory_size,
        source_bits,
        parent_id,
    })
}

/// Returns whether a Sound object of a bank of `version` has a layout this reader knows.
pub(crate) const fn has_sound_layout(version: u32) -> bool {
    version >= FIRST_SOUND_VERSION
}
