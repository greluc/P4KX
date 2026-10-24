// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::TextureGroups;

const NONE: u32 = u32::MAX;

/// The split textures of an Archive by Entry id: the header and the parts of each, and the sizes
/// of the whole texture.
#[derive(Debug, Default)]
pub(crate) struct EntryGroups {
    group_of: Vec<u32>,
    groups: Vec<Group>,
    parts: usize,
}

#[derive(Debug)]
struct Group {
    members: Box<[u32]>,
    uncompressed_size: u64,
}

impl EntryGroups {
    /// Translates `groups`, which are over Archive positions, to Entry ids; `id_of` is indexed
    /// by position and `len` is the number of Entries.
    pub(crate) fn new(groups: &TextureGroups, id_of: &[u32], len: usize) -> Self {
        let mut group_of = vec![NONE; len];
        let mut translated = Vec::with_capacity(groups.len());
        let mut parts = 0;
        for (number, group) in groups.groups().iter().enumerate() {
            let members: Box<[u32]> = group
                .members()
                .filter_map(|position| id_of.get(position).copied())
                .collect();
            for &id in &members {
                if let Some(slot) = group_of.get_mut(id as usize) {
                    *slot = u32::try_from(number).unwrap_or(NONE);
                }
            }
            parts += members.len().saturating_sub(1);
            translated.push(Group {
                members,
                uncompressed_size: group.uncompressed_size(),
            });
        }
        Self {
            group_of,
            groups: translated,
            parts,
        }
    }

    fn group(&self, id: u32) -> Option<&Group> {
        let number = *self.group_of.get(id as usize)?;
        self.groups.get(number as usize)
    }

    /// Returns the Entries of the texture `id` belongs to, the header first, or `None`.
    pub(crate) fn members(&self, id: u32) -> Option<&[u32]> {
        self.group(id).map(|group| &*group.members)
    }

    /// Returns the parts of the texture whose header is `id`; empty for any other Entry.
    pub(crate) fn parts_of(&self, id: u32) -> &[u32] {
        match self.group(id).map(|group| &*group.members) {
            Some([header, parts @ ..]) if *header == id => parts,
            _ => &[],
        }
    }

    /// Returns whether `id` is a part or alpha tail Entry of a texture.
    pub(crate) fn is_part(&self, id: u32) -> bool {
        matches!(self.group(id).map(|group| &*group.members), Some([header, ..]) if *header != id)
    }

    /// Returns the size of the texture whose header is `id` after decompression.
    pub(crate) fn total_size(&self, id: u32) -> Option<u64> {
        let group = self.group(id)?;
        (group.members.first() == Some(&id)).then_some(group.uncompressed_size)
    }

    /// Returns the number of Entries folded into the row of their header.
    pub(crate) const fn part_count(&self) -> usize {
        self.parts
    }

    /// Marks every member of each texture that has a marked member, in `marked` by id.
    pub(crate) fn complete(&self, marked: &mut [bool]) {
        for group in &self.groups {
            if group
                .members
                .iter()
                .any(|&id| marked.get(id as usize).copied().unwrap_or(false))
            {
                for &id in &group.members {
                    if let Some(slot) = marked.get_mut(id as usize) {
                        *slot = true;
                    }
                }
            }
        }
    }
}
