// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::Guid;
use p4kx_screens::{EntityScreens, Screen};

use super::contents::UiContents;
use super::job::{Job, Poll};
use crate::entities::ResolveFailure;

type Listing = Result<EntityScreens, ResolveFailure>;

/// Where listing the screens of a picked ship stands.
#[derive(Debug, Clone, Copy)]
pub enum UiScreensStatus<'a> {
    /// The Loadout is resolved and the screens are listed off the UI thread.
    Resolving,
    /// The Archive changed on disk, so the screens were not listed.
    ArchiveChanged,
    /// Listing failed for the reason given, or stopped without a result.
    Failed(Option<&'a str>),
    /// The DataForge has no Record of the ship.
    NoRecord(Guid),
    /// The screens are listed.
    Ready(&'a EntityScreens),
}

enum State {
    Resolving(Job<Listing>),
    ArchiveChanged,
    Failed(Option<String>),
    NoRecord(Guid),
    Ready {
        listing: Box<EntityScreens>,
        picked: Option<usize>,
        contents: Option<UiContents>,
    },
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Resolving(_) => "Resolving",
            Self::ArchiveChanged => "ArchiveChanged",
            Self::Failed(_) => "Failed",
            Self::NoRecord(_) => "NoRecord",
            Self::Ready { .. } => "Ready",
        })
    }
}

/// The ship or vehicle picked in the tab and the screens of its default Loadout.
#[derive(Debug)]
pub struct ShipPick {
    guid: Guid,
    class_name: String,
    state: State,
}

impl ShipPick {
    pub(super) fn new(guid: Guid, class_name: &str, job: Result<Job<Listing>, String>) -> Self {
        Self {
            guid,
            class_name: class_name.to_owned(),
            state: match job {
                Ok(job) => State::Resolving(job),
                Err(reason) => State::Failed(Some(reason)),
            },
        }
    }

    /// Returns the GUID of the Entity.
    #[must_use]
    pub const fn guid(&self) -> Guid {
        self.guid
    }

    /// Returns the class name of the Entity.
    #[must_use]
    pub fn class_name(&self) -> &str {
        &self.class_name
    }

    /// Returns where listing the screens stands.
    #[must_use]
    pub fn status(&self) -> UiScreensStatus<'_> {
        match &self.state {
            State::Resolving(_) => UiScreensStatus::Resolving,
            State::ArchiveChanged => UiScreensStatus::ArchiveChanged,
            State::Failed(reason) => UiScreensStatus::Failed(reason.as_deref()),
            State::NoRecord(guid) => UiScreensStatus::NoRecord(*guid),
            State::Ready { listing, .. } => UiScreensStatus::Ready(listing),
        }
    }

    /// Returns the position of the picked screen in the listing.
    #[must_use]
    pub const fn picked(&self) -> Option<usize> {
        match &self.state {
            State::Ready { picked, .. } => *picked,
            _ => None,
        }
    }

    /// Returns the picked screen.
    #[must_use]
    pub fn screen(&self) -> Option<&Screen> {
        match &self.state {
            State::Ready {
                listing,
                picked: Some(picked),
                ..
            } => listing.screens.get(*picked),
            _ => None,
        }
    }

    /// Returns the manufacturer code whose colours the screens are drawn in.
    #[must_use]
    pub fn brand(&self) -> Option<&str> {
        match &self.state {
            State::Ready { listing, .. } => listing.brand.as_deref(),
            _ => None,
        }
    }

    /// Returns what the picked screen is made of.
    #[must_use]
    pub const fn contents(&self) -> Option<&UiContents> {
        match &self.state {
            State::Ready { contents, .. } => contents.as_ref(),
            _ => None,
        }
    }

    pub(super) fn pick(&mut self, position: usize) {
        if let State::Ready {
            listing,
            picked,
            contents,
        } = &mut self.state
            && let Some(screen) = listing.screens.get(position)
        {
            *picked = Some(position);
            *contents = Some(UiContents::of_screen(screen));
        }
    }

    pub(super) fn poll(&mut self) -> bool {
        let State::Resolving(job) = &self.state else {
            return false;
        };
        let listing = match job.poll() {
            Poll::Pending => return false,
            Poll::Ready(Ok(listing)) => listing,
            Poll::Ready(Err(ResolveFailure::ArchiveChanged)) => {
                self.state = State::ArchiveChanged;
                return true;
            }
            Poll::Ready(Err(ResolveFailure::Failed(reason))) => {
                self.state = State::Failed(Some(reason));
                return true;
            }
            Poll::Ready(Err(ResolveFailure::NoRecord(guid))) => {
                self.state = State::NoRecord(guid);
                return true;
            }
            Poll::Ready(Err(ResolveFailure::Cancelled | ResolveFailure::Lost)) | Poll::Lost => {
                self.state = State::Failed(None);
                return true;
            }
        };
        self.state = State::Ready {
            listing: Box::new(listing),
            picked: None,
            contents: None,
        };
        self.pick(0);
        true
    }
}
