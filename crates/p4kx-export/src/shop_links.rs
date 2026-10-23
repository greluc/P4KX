use std::collections::HashMap;

use crate::contract::StarmapObject;
use crate::guid_text::CatalogGuid;

/// The starmap objects by the lower-case names that place them.
pub(crate) struct Links {
    by_name: HashMap<String, Option<CatalogGuid>>,
}

impl Links {
    pub(crate) fn new(starmap: &[StarmapObject<'_>]) -> Self {
        let mut by_name: HashMap<String, Option<CatalogGuid>> = HashMap::new();
        for object in starmap {
            let names = [Some(object.class_name), object.entity_name.as_deref()];
            for name in names.into_iter().flatten().filter(|name| !name.is_empty()) {
                by_name
                    .entry(name.to_ascii_lowercase())
                    .and_modify(|held| {
                        if *held != Some(object.guid) {
                            *held = None;
                        }
                    })
                    .or_insert(Some(object.guid));
            }
        }
        Self { by_name }
    }

    pub(crate) fn find(&self, name: &str) -> Option<CatalogGuid> {
        self.by_name
            .get(&name.to_ascii_lowercase())
            .copied()
            .flatten()
    }

    pub(crate) fn is_ambiguous(&self, name: &str) -> bool {
        matches!(self.by_name.get(&name.to_ascii_lowercase()), Some(None))
    }
}
