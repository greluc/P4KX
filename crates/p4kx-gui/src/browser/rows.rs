use crate::Browser;

/// What a row of the Entry table shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryRowKind {
    /// An Entry that is no part of a split texture.
    Entry,
    /// The `.dds` header of a split texture, its row standing for the header and its parts.
    Texture {
        /// The number of parts and alpha tail Entries folded into the row.
        parts: usize,
    },
    /// A part or alpha tail Entry shown below its expanded texture.
    Part,
}

/// One row of the Entry table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryRow {
    /// The Entry of the row.
    pub id: u32,
    /// What the row shows.
    pub kind: EntryRowKind,
}

impl Browser {
    /// Returns the rows of the Entry table: the shown Entries, and below each expanded texture
    /// its parts.
    #[must_use]
    pub fn rows(&self) -> &[EntryRow] {
        &self.rows
    }

    /// Returns whether the parts of the texture whose header is `id` are shown below it.
    #[must_use]
    pub fn is_texture_expanded(&self, id: u32) -> bool {
        self.expanded_textures.contains(&id)
    }

    /// Shows the parts of the texture whose header is `id` below it, or hides them again.
    pub fn toggle_texture(&mut self, id: u32) {
        if !self.expanded_textures.remove(&id) {
            self.expanded_textures.insert(id);
        }
        self.refresh_rows();
    }

    pub(super) fn shown_with_parts(&self) -> Vec<u32> {
        let index = self.shown_index();
        let mut ids = Vec::with_capacity(self.view_len);
        for &id in &self.shown {
            ids.push(id);
            ids.extend_from_slice(index.texture_parts(id));
        }
        ids
    }

    pub(super) fn refresh_rows(&mut self) {
        let index = self.shown_index();
        let mut rows = Vec::with_capacity(self.shown.len());
        for &id in &self.shown {
            let parts = index.texture_parts(id);
            if parts.is_empty() {
                rows.push(EntryRow {
                    id,
                    kind: EntryRowKind::Entry,
                });
                continue;
            }
            rows.push(EntryRow {
                id,
                kind: EntryRowKind::Texture { parts: parts.len() },
            });
            if self.expanded_textures.contains(&id) {
                rows.extend(parts.iter().map(|&part| EntryRow {
                    id: part,
                    kind: EntryRowKind::Part,
                }));
            }
        }
        self.rows = rows;
    }
}
