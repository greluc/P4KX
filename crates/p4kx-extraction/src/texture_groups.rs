use p4kx_archive::Archive;
use rayon::prelude::*;

use crate::PathIndex;
use crate::texture_job::{Member, Role, role};

const NONE: u32 = u32::MAX;

/// The Entries of one split texture: its `.dds` header and every part and alpha tail Entry the
/// Archive holds for it, as positions in [`Archive::entries`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureGroup {
    header: u32,
    parts: Vec<u32>,
    uncompressed_size: u64,
    compressed_size: u64,
}

impl TextureGroup {
    /// Returns the position of the `.dds` header Entry.
    #[must_use]
    pub const fn header(&self) -> usize {
        self.header as usize
    }

    /// Returns the positions of the parts and alpha tail Entries: the parts `.dds.1` … `.dds.N`
    /// by number, then the alpha tail `.dds.a` and its parts `.dds.1a` … `.dds.Na`.
    pub fn parts(&self) -> impl ExactSizeIterator<Item = usize> + '_ {
        self.parts.iter().map(|&position| position as usize)
    }

    /// Returns the number of Entries of the group, the header included.
    #[must_use]
    pub const fn entry_count(&self) -> usize {
        self.parts.len() + 1
    }

    /// Returns the positions of all Entries of the group, the header first.
    pub fn members(&self) -> impl Iterator<Item = usize> + '_ {
        std::iter::once(self.header()).chain(self.parts())
    }

    /// Returns the sum of the sizes of the Entries after decompression.
    #[must_use]
    pub const fn uncompressed_size(&self) -> u64 {
        self.uncompressed_size
    }

    /// Returns the sum of the sizes of the Entries as stored.
    #[must_use]
    pub const fn compressed_size(&self) -> u64 {
        self.compressed_size
    }
}

/// The split textures of an Archive: each `.dds` header Entry with its parts `.dds.N` and its
/// alpha tail `.dds.a` with parts `.dds.Na`.
///
/// The grouping is computed once, in one parallel pass over the Archive. A part or alpha tail
/// without a header Entry belongs to no group, and a header without parts is no group.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextureGroups {
    group_of: Vec<u32>,
    groups: Vec<TextureGroup>,
}

impl TextureGroups {
    /// Groups the texture Entries of `archive`.
    #[must_use]
    pub fn build(archive: &Archive) -> Self {
        let entries = archive.entries();
        let index = PathIndex::new(archive);
        let mut found: Vec<(u32, (bool, u32), u32)> = entries
            .par_iter()
            .enumerate()
            .filter_map(|(position, entry)| {
                let Some(Role::Member(header_path, member)) = role(entry.path()) else {
                    return None;
                };
                let header = u32::try_from(index.position(header_path)?).ok()?;
                Some((header, order(member), u32::try_from(position).ok()?))
            })
            .collect();
        let mut group_of = vec![NONE; entries.len()];
        let mut groups: Vec<TextureGroup> = Vec::new();
        if found.is_empty() {
            return Self { group_of, groups };
        }
        found.par_sort_unstable();
        for (header, _, part) in found {
            if groups.last().is_none_or(|group| group.header != header) {
                groups.push(TextureGroup {
                    header,
                    parts: Vec::new(),
                    uncompressed_size: 0,
                    compressed_size: 0,
                });
            }
            if let Some(group) = groups.last_mut() {
                group.parts.push(part);
            }
        }
        for (number, group) in groups.iter_mut().enumerate() {
            let number = u32::try_from(number).unwrap_or(NONE);
            let members: Vec<usize> = group.members().collect();
            for position in members {
                if let Some(entry) = entries.get(position) {
                    group.uncompressed_size = group
                        .uncompressed_size
                        .saturating_add(entry.uncompressed_size());
                    group.compressed_size = group
                        .compressed_size
                        .saturating_add(entry.compressed_size());
                }
                if let Some(slot) = group_of.get_mut(position) {
                    *slot = number;
                }
            }
        }
        Self { group_of, groups }
    }

    /// Returns the groups, ordered by the position of their header.
    #[must_use]
    pub fn groups(&self) -> &[TextureGroup] {
        &self.groups
    }

    /// Returns the number of groups.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.groups.len()
    }

    /// Returns whether the Archive holds no split texture.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// Returns the number of the group the Entry at `position` belongs to, as an index into
    /// [`TextureGroups::groups`].
    #[must_use]
    pub fn group_number(&self, position: usize) -> Option<usize> {
        self.group_of
            .get(position)
            .filter(|&&number| number != NONE)
            .map(|&number| number as usize)
    }

    /// Returns the group the Entry at `position` belongs to, as header or as part.
    #[must_use]
    pub fn group_of(&self, position: usize) -> Option<&TextureGroup> {
        self.groups.get(self.group_number(position)?)
    }

    /// Returns whether the Entry at `position` is a part or alpha tail Entry of a group, rather
    /// than its header or an Entry of no group.
    #[must_use]
    pub fn is_part(&self, position: usize) -> bool {
        self.group_of(position)
            .is_some_and(|group| group.header() != position)
    }

    /// Marks every Entry of each group that has a marked Entry, in `marked`, which is indexed by
    /// position.
    pub fn complete(&self, marked: &mut [bool]) {
        for group in &self.groups {
            if group
                .members()
                .any(|position| marked.get(position).copied().unwrap_or(false))
            {
                for position in group.members() {
                    if let Some(slot) = marked.get_mut(position) {
                        *slot = true;
                    }
                }
            }
        }
    }
}

const fn order(member: Member) -> (bool, u32) {
    match member {
        Member::Part(number) => (false, number),
        Member::AlphaHeader => (true, 0),
        Member::AlphaPart(number) => (true, number),
    }
}
