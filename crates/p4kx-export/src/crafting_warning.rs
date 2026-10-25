// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;

/// Something that makes the crafting data incomplete or ambiguous without failing a Record.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CraftingWarning {
    /// The DataForge holds more References than the reference index keeps, so
    /// [`Crafting::reward_sources`](crate::Crafting::reward_sources) may lack places that reward
    /// a pool.
    RewardSourcesTruncated,
    /// This many Records could not be read by the reference index; a reward source in one of
    /// them is missing from [`Crafting::reward_sources`](crate::Crafting::reward_sources).
    UnreadableReferenceRecords(usize),
    /// The DataForge holds this many dismantle processes instead of one, so
    /// [`Crafting::dismantle_return`](crate::Crafting::dismantle_return) links no blueprint.
    AmbiguousDismantle(usize),
    /// This many ships have a vehicle implementation file or Modification patch file that exists
    /// but cannot be read, so no paint is assigned through it. `details` names the files and why
    /// they cannot be read, ordered and without repeats. A file the Archive lacks is no such ship.
    UnreadableVehicleFiles {
        /// The number of ships.
        count: usize,
        /// One entry per file: its path and the reason.
        details: Vec<String>,
    },
    /// This many references to containers that cannot be read, Child elements with a malformed or
    /// out-of-range placement and walk limits were hit, so the starmap objects below them have no
    /// position. A container the Archive lacks is not counted.
    UnplacedStarmapContainers(usize),
    /// The shop registry exists but cannot be read, so `shops` is empty although the Build has
    /// shops.
    UnreadableShopRegistry,
    /// The Archive holds starmap objects but no shop registry at the known path, so `shops` is
    /// empty; the registry may have moved in this Build.
    MissingShopRegistry,
    /// This many shop containers cannot be opened or their `entitycomponentshop.xml` cannot be
    /// read, so the shops they hold may lack kind and brand.
    UnreadableShopContainers(usize),
    /// This many inventory files that shop containers name cannot be read, so the shops that use
    /// them have no item list. `details` names the files and why they cannot be read, ordered and
    /// without repeats.
    UnreadableShopInventories {
        /// The number of files.
        count: usize,
        /// One entry per file: its path and the reason.
        details: Vec<String>,
    },
    /// This many distinct inventory files that shop containers name are not in the Archive, so
    /// the shops that use them have no item list; the files may have moved in this Build.
    MissingShopInventories(usize),
    /// Shop containers name shops, but no `.soc` holds a shop component, so no shop has an item
    /// list; the component layout may have changed in this Build.
    NoShopComponents,
}

const LISTED_DETAILS: usize = 8;

/// The details of a warning as ` (a; b)`, empty for none; lists at most [`LISTED_DETAILS`].
struct Listed<'a>(&'a [String]);

impl fmt::Display for Listed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(first) = self.0.first() else {
            return Ok(());
        };
        write!(f, " ({first}")?;
        for detail in self.0.iter().take(LISTED_DETAILS).skip(1) {
            write!(f, "; {detail}")?;
        }
        match self.0.len().checked_sub(LISTED_DETAILS) {
            Some(more) if more > 0 => write!(f, "; and {more} more)"),
            _ => f.write_str(")"),
        }
    }
}

impl fmt::Display for CraftingWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RewardSourcesTruncated => f.write_str(
                "the reference index is truncated; the reward sources of the blueprint pools are incomplete",
            ),
            Self::UnreadableReferenceRecords(count) => write!(
                f,
                "{count} Records cannot be read by the reference index; their reward sources are missing"
            ),
            Self::AmbiguousDismantle(count) => write!(
                f,
                "the DataForge holds {count} dismantle processes instead of one; no dismantle return is linked"
            ),
            Self::UnreadableVehicleFiles { count, details } => write!(
                f,
                "{count} ships have a vehicle file that cannot be read; their paints are missing{}",
                Listed(details)
            ),
            Self::UnplacedStarmapContainers(count) => write!(
                f,
                "{count} system containers or placements cannot be read; the starmap objects below them have no position"
            ),
            Self::UnreadableShopRegistry => f.write_str(
                "the shop registry cannot be read; the catalog has no shops although the Build has some",
            ),
            Self::MissingShopRegistry => f.write_str(
                "the Archive holds no shop registry at the known path; the catalog has no shops",
            ),
            Self::UnreadableShopContainers(count) => write!(
                f,
                "{count} shop containers cannot be read; the shops in them may lack kind and brand"
            ),
            Self::UnreadableShopInventories { count, details } => write!(
                f,
                "{count} shop inventory files cannot be read; the shops that use them have no item list{}",
                Listed(details)
            ),
            Self::MissingShopInventories(count) => write!(
                f,
                "{count} shop inventory files that shop containers name are not in the Archive; the shops that use them have no item list"
            ),
            Self::NoShopComponents => f.write_str(
                "no shop component was found in the .soc files of the shop containers; no shop has an item list",
            ),
        }
    }
}
