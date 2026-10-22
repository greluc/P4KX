use std::fmt;

/// Something that makes the crafting data incomplete or ambiguous without failing a Record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// but cannot be read, so no paint is assigned through it.
    UnreadableVehicleFiles(usize),
    /// This many references to containers that cannot be read, Child elements with a malformed or
    /// out-of-range placement and walk limits were hit, so the starmap objects below them have no
    /// position. A container the Archive lacks is not counted.
    UnplacedStarmapContainers(usize),
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
            Self::UnreadableVehicleFiles(count) => write!(
                f,
                "{count} ships have a vehicle file that cannot be read; their paints are missing"
            ),
            Self::UnplacedStarmapContainers(count) => write!(
                f,
                "{count} system containers or placements cannot be read; the starmap objects below them have no position"
            ),
        }
    }
}
