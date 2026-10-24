// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::Browser;
use crate::containers::Place;
use crate::entry_index::is_container;
use crate::sort::file_name;

/// A container Entry selected in the browser, as the Export of its Interior needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectedContainer {
    /// The Entry paths from the Archive down to the container, each in the Archive or container
    /// before it.
    pub(crate) chain: Vec<String>,
    /// The container's file name without its extension.
    pub(crate) name: String,
}

impl Browser {
    /// Returns the container Entry selected for the preview, with where it lies, or `None` when
    /// the selected Entry is no `.socpak` or the places around it are not open.
    pub(crate) fn selected_container(&self) -> Option<SelectedContainer> {
        let preview = self.preview()?;
        let path = self.index().entry(preview.entry())?.path();
        if !is_container(path) {
            return None;
        }
        let mut chain = vec![path.to_owned()];
        let mut place = self.place;
        while let Place::Container(id) = place {
            let (parent, entry) = self.containers.origin(id)?;
            let index = self.containers.index_of(parent, &self.index)?;
            chain.push(index.entry(entry)?.path().to_owned());
            place = parent;
        }
        chain.reverse();
        let name = file_name(path);
        let name = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        Some(SelectedContainer {
            chain,
            name: name.to_owned(),
        })
    }
}
