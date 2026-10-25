// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::Path;

use crate::channel_name;

/// The kind of a Channel, told by the name of the installation folder that holds its Archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelKind {
    /// The public release, `LIVE`.
    Live,
    /// The public test universe, `PTU`.
    Ptu,
    /// The evocati test universe, `EPTU`.
    Eptu,
    /// A hotfix build, `HOTFIX`.
    Hotfix,
    /// The technical preview, `TECH-PREVIEW`.
    TechPreview,
    /// Any other folder name.
    Unknown,
}

impl ChannelKind {
    /// Returns the kind named by `name`, ignoring ASCII case.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        const NAMES: [(&str, ChannelKind); 5] = [
            ("LIVE", ChannelKind::Live),
            ("PTU", ChannelKind::Ptu),
            ("EPTU", ChannelKind::Eptu),
            ("HOTFIX", ChannelKind::Hotfix),
            ("TECH-PREVIEW", ChannelKind::TechPreview),
        ];
        NAMES
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(name))
            .map_or(Self::Unknown, |&(_, kind)| kind)
    }

    /// Returns the kind of the Archive at `archive`, told by the name of its folder.
    #[must_use]
    pub fn of_archive(archive: &Path) -> Self {
        channel_name(archive).map_or(Self::Unknown, |name| Self::from_name(&name))
    }

    /// Returns the canonical Channel name, or `None` for an unknown kind.
    #[must_use]
    pub const fn name(self) -> Option<&'static str> {
        match self {
            Self::Live => Some("LIVE"),
            Self::Ptu => Some("PTU"),
            Self::Eptu => Some("EPTU"),
            Self::Hotfix => Some("HOTFIX"),
            Self::TechPreview => Some("TECH-PREVIEW"),
            Self::Unknown => None,
        }
    }

    /// Returns whether Archives of this kind hold material of a Star Citizen test build, whose
    /// publication the Terms of Service forbid. `LIVE` and unknown kinds hold none.
    #[must_use]
    pub const fn is_test_build(self) -> bool {
        matches!(
            self,
            Self::Ptu | Self::Eptu | Self::Hotfix | Self::TechPreview
        )
    }

    /// Returns the English notice for an Archive of this kind, or `None` when it needs none.
    #[must_use]
    pub fn notice(self) -> Option<String> {
        let name = self.name().filter(|_| self.is_test_build())?;
        Some(format!(
            "This Archive belongs to the {name} Channel, which carries Star Citizen test builds. \
             Material of test builds is confidential under the Star Citizen Terms of Service: do \
             not share or publish anything taken from it."
        ))
    }
}
