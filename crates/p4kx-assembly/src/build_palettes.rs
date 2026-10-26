// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use p4kx_dataforge::Guid;
use p4kx_model::{MaterialStatus, TintPalette};

use crate::{Geometry, IssueKind, PaletteFailure, PaletteProvider};

type Cached = Result<Arc<TintPalette>, PaletteFailure>;

/// Resolves the tint palette of each part, reading every distinct palette Record once.
///
/// A part takes the palette its Entity's geometry names; when that cannot be resolved, or the
/// Entity names none, the default palette of its material library. Palettes are cached by the
/// GUID of the Record, however it was named, so one Record is one palette. Each failure message
/// is reported for the first part that meets it.
pub(crate) struct Palettes<'p> {
    provider: Option<&'p dyn PaletteProvider>,
    cache: HashMap<Guid, Cached>,
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
        self.of_named(geometry.and_then(Geometry::palette), library)
    }

    /// Returns the palette of a part drawn with `library` that names the palette `named`, and what
    /// could not be resolved on the way.
    pub(crate) fn of_named(
        &mut self,
        named: Option<&crate::PaletteRef>,
        library: &MaterialStatus,
    ) -> (Option<Arc<TintPalette>>, Vec<IssueKind>) {
        let mut issues = Vec::new();
        let Some(provider) = self.provider else {
            return (None, issues);
        };
        if let Some(named) = named {
            let guid = named.guid();
            let what = format!("the tint palette {guid}");
            let found = if named.child_path().is_empty() {
                self.read(provider, guid)
            } else {
                Err(PaletteFailure::Unreadable {
                    reason: format!("the child {} is not supported", named.child_path()),
                })
            };
            match found {
                Ok(palette) => return (Some(palette), issues),
                Err(failure) => self.note(&mut issues, &what, &failure),
            }
        }
        let default = library
            .library()
            .and_then(|library| library.default_palette.as_deref());
        if let Some(path) = default {
            let what = format!("the default tint palette {path} of the material library");
            match provider
                .guid_at(path)
                .and_then(|guid| self.read(provider, guid))
            {
                Ok(palette) => return (Some(palette), issues),
                Err(failure) => self.note(&mut issues, &what, &failure),
            }
        }
        (None, issues)
    }

    fn read(&mut self, provider: &dyn PaletteProvider, guid: Guid) -> Cached {
        self.cache
            .entry(guid)
            .or_insert_with(|| provider.by_guid(guid).map(Arc::new))
            .clone()
    }

    fn note(&mut self, issues: &mut Vec<IssueKind>, what: &str, failure: &PaletteFailure) {
        let reason = match failure {
            PaletteFailure::Missing => format!("{what} does not exist"),
            PaletteFailure::Unreadable { reason } => format!("{what} cannot be used: {reason}"),
        };
        if self.reported.insert(reason.clone()) {
            issues.push(IssueKind::Palette { reason });
        }
    }
}
