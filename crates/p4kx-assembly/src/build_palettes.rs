// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use p4kx_model::{MaterialStatus, TintPalette};

use crate::{Geometry, IssueKind, PaletteFailure, PaletteProvider};

type Cached = Result<Arc<TintPalette>, PaletteFailure>;

/// Resolves the tint palette of each part, reading every distinct palette once.
///
/// A part takes the palette its Entity's geometry names; when that cannot be resolved, or the
/// Entity names none, the default palette of its material library. Each failure is reported for
/// the first part that meets it.
pub(crate) struct Palettes<'p> {
    provider: Option<&'p dyn PaletteProvider>,
    cache: HashMap<String, Cached>,
    reported: HashSet<String>,
}

impl<'p> Palettes<'p> {
    pub(crate) fn new(provider: Option<&'p dyn PaletteProvider>) -> Self {
        Self {
            provider,
            cache: HashMap::new(),
            reported: HashSet::new(),
        }
    }

    /// Returns the palette of a part drawn with `library`, and what could not be resolved on the
    /// way.
    pub(crate) fn of(
        &mut self,
        geometry: Option<&Geometry>,
        library: &MaterialStatus,
    ) -> (Option<Arc<TintPalette>>, Vec<IssueKind>) {
        let mut issues = Vec::new();
        if self.provider.is_none() {
            return (None, issues);
        }
        if let Some(named) = geometry.and_then(Geometry::palette) {
            let guid = named.guid();
            let key = format!("guid:{guid}");
            let found = if named.child_path().is_empty() {
                self.read(&key, |provider| provider.by_guid(guid))
            } else {
                Err(PaletteFailure::Unreadable {
                    reason: format!("the child {} is not supported", named.child_path()),
                })
            };
            match found {
                Ok(palette) => return (Some(palette), issues),
                Err(failure) => {
                    if self.reported.insert(key) {
                        issues.push(IssueKind::Palette {
                            reason: describe(&format!("the tint palette {guid}"), &failure),
                        });
                    }
                }
            }
        }
        let default = library
            .library()
            .and_then(|library| library.default_palette.as_deref());
        if let Some(path) = default {
            let key = format!("path:{}", path.to_ascii_lowercase());
            match self.read(&key, |provider| provider.by_path(path)) {
                Ok(palette) => return (Some(palette), issues),
                Err(failure) => {
                    if self.reported.insert(key) {
                        issues.push(IssueKind::Palette {
                            reason: describe(
                                &format!("the default tint palette {path} of the material library"),
                                &failure,
                            ),
                        });
                    }
                }
            }
        }
        (None, issues)
    }

    fn read(
        &mut self,
        key: &str,
        fetch: impl FnOnce(&dyn PaletteProvider) -> Result<TintPalette, PaletteFailure>,
    ) -> Cached {
        if let Some(cached) = self.cache.get(key) {
            return cached.clone();
        }
        let result = self
            .provider
            .ok_or(PaletteFailure::Missing)
            .and_then(fetch)
            .map(Arc::new);
        self.cache.insert(key.to_owned(), result.clone());
        result
    }
}

fn describe(what: &str, failure: &PaletteFailure) -> String {
    match failure {
        PaletteFailure::Missing => format!("{what} does not exist"),
        PaletteFailure::Unreadable { reason } => format!("{what} cannot be used: {reason}"),
    }
}
