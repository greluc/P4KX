use std::collections::HashMap;

use p4kx_extraction::{Guid, TagTree};

use crate::guid_text::CatalogGuid;

/// The tags of a tag tree by name, to turn the tag names a Record stores into tag GUIDs.
///
/// Names compare ignoring ASCII case. A name that more than one tag of the tree has, and a name
/// no tag has, resolve to nothing: a GUID is never guessed.
pub(crate) struct TagNames {
    by_name: HashMap<String, Option<Guid>>,
}

impl TagNames {
    /// Returns the names of the tags in `tree`.
    pub(crate) fn new(tree: &TagTree) -> Self {
        let mut by_name: HashMap<String, Option<Guid>> = HashMap::with_capacity(tree.len());
        for tag in tree.iter() {
            by_name
                .entry(tag.name().to_ascii_lowercase())
                .and_modify(|found| *found = None)
                .or_insert_with(|| Some(tag.guid()));
        }
        Self { by_name }
    }

    /// Returns the GUID of the only tag named `name`; `None` for an unknown or ambiguous name.
    /// `buffer` holds the lower-case name between calls.
    pub(crate) fn resolve(&self, name: &str, buffer: &mut String) -> Option<CatalogGuid> {
        buffer.clear();
        buffer.extend(name.chars().map(|c| c.to_ascii_lowercase()));
        self.by_name
            .get(buffer.as_str())
            .copied()
            .flatten()
            .map(CatalogGuid)
    }

    /// Returns the GUIDs of `names`, one per name in the same order, `None` where a name does not
    /// resolve.
    pub(crate) fn resolve_all<S: AsRef<str>>(
        &self,
        names: &[S],
        buffer: &mut String,
    ) -> Vec<Option<CatalogGuid>> {
        names
            .iter()
            .map(|name| self.resolve(name.as_ref(), buffer))
            .collect()
    }
}
