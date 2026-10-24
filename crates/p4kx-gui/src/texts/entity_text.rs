// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// A text of the Entities tab, looked up in the Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityText {
    /// The tab.
    Tab,
    /// The label of the search field.
    Label,
    /// The hint inside the empty search field.
    Hint,
    /// The title of the empty state before an Archive is open.
    EmptyTitle,
    /// The hint of the empty state.
    EmptyHint,
    /// Shown while the Entities are listed.
    Loading,
    /// Shown when the DataForge holds no ship, vehicle or weapon.
    NoEntities,
    /// Listing failed; takes `reason`.
    Failed,
    /// Listing stopped without a result.
    Stopped,
    /// The Archive changed on disk.
    ArchiveChanged,
    /// The filter for every kind.
    FilterAll,
    /// The filter for ships.
    FilterShips,
    /// The filter for ground vehicles.
    FilterVehicles,
    /// The filter for weapons.
    FilterWeapons,
    /// The number of Entities; takes `count` and `number`.
    Count,
    /// The number of Entities when the search leaves some; takes `count`, `number` and `total`.
    CountShown,
    /// Shown when no Entity matches the search.
    NoMatch,
    /// The heading of the Loadout tree.
    LoadoutHeading,
    /// Shown while no Entity is selected.
    LoadoutNone,
    /// Shown while the Loadout is resolved.
    LoadoutResolving,
    /// The Loadout cannot be resolved; takes `reason`.
    LoadoutFailed,
    /// Resolving the Loadout stopped without a result.
    LoadoutStopped,
    /// The Archive changed on disk, so the Loadout was not resolved.
    LoadoutArchiveChanged,
    /// Shown when the Entity has no ports.
    LoadoutNoPorts,
    /// What the changes amount to; takes `excluded` and `replaced`.
    Changes,
    /// Shown while the Loadout has no changes.
    NoChanges,
    /// The action that takes back every change.
    Reset,
    /// The action that opens the Export of the Entity as GLB.
    Export,
    /// The accessible name of the checkbox that includes a port; takes `port`.
    Include,
    /// The action that opens the choice of a replacement.
    Replace,
    /// The action that takes back the change to a port.
    Restore,
    /// The port is left out.
    StatusExcluded,
    /// The port holds another Entity.
    StatusReplaced,
    /// The port holds a docked vehicle.
    StatusDocked,
    /// The port holds nothing.
    StatusEmpty,
    /// The attachment of the port cannot be resolved.
    StatusUnresolved,
    /// Expands a port; takes `name`.
    ExpandPort,
    /// Collapses a port; takes `name`.
    CollapsePort,
    /// The title of the choice of a replacement; takes `port`.
    ReplaceTitle,
    /// What the port takes, item types and sizes; takes `types` and `sizes`.
    ReplaceTakes,
    /// What the port takes, item types only; takes `types`.
    ReplaceTakesTypes,
    /// What the port takes, sizes only; takes `sizes`.
    ReplaceTakesSizes,
    /// The port takes any Entity.
    ReplaceTakesAny,
    /// The hint inside the search field of the choice.
    ReplaceHint,
    /// How many Entities fit; takes `number` and `total`.
    ReplaceCount,
    /// Shown when no Entity fits the port and the search.
    ReplaceNone,
    /// The item type and size of an Entity; takes `type` and `size`.
    FactsSized,
    /// The item type of an Entity that gives no size; takes `type`.
    Facts,
    /// Shown for an Entity without an item type.
    FactsNone,
}

impl EntityText {
    /// Every text, for checks of the Fluent files.
    pub(crate) const ALL: [Self; 49] = [
        Self::Tab,
        Self::Label,
        Self::Hint,
        Self::EmptyTitle,
        Self::EmptyHint,
        Self::Loading,
        Self::NoEntities,
        Self::Failed,
        Self::Stopped,
        Self::ArchiveChanged,
        Self::FilterAll,
        Self::FilterShips,
        Self::FilterVehicles,
        Self::FilterWeapons,
        Self::Count,
        Self::CountShown,
        Self::NoMatch,
        Self::LoadoutHeading,
        Self::LoadoutNone,
        Self::LoadoutResolving,
        Self::LoadoutFailed,
        Self::LoadoutStopped,
        Self::LoadoutArchiveChanged,
        Self::LoadoutNoPorts,
        Self::Changes,
        Self::NoChanges,
        Self::Reset,
        Self::Export,
        Self::Include,
        Self::Replace,
        Self::Restore,
        Self::StatusExcluded,
        Self::StatusReplaced,
        Self::StatusDocked,
        Self::StatusEmpty,
        Self::StatusUnresolved,
        Self::ExpandPort,
        Self::CollapsePort,
        Self::ReplaceTitle,
        Self::ReplaceTakes,
        Self::ReplaceTakesTypes,
        Self::ReplaceTakesSizes,
        Self::ReplaceTakesAny,
        Self::ReplaceHint,
        Self::ReplaceCount,
        Self::ReplaceNone,
        Self::FactsSized,
        Self::Facts,
        Self::FactsNone,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Tab => "entities-tab",
            Self::Label => "entities-label",
            Self::Hint => "entities-hint",
            Self::EmptyTitle => "entities-empty-title",
            Self::EmptyHint => "entities-empty-hint",
            Self::Loading => "entities-loading",
            Self::NoEntities => "entities-none",
            Self::Failed => "entities-failed",
            Self::Stopped => "entities-stopped",
            Self::ArchiveChanged => "entities-archive-changed",
            Self::FilterAll => "entities-filter-all",
            Self::FilterShips => "entities-filter-ships",
            Self::FilterVehicles => "entities-filter-vehicles",
            Self::FilterWeapons => "entities-filter-weapons",
            Self::Count => "entities-count",
            Self::CountShown => "entities-count-shown",
            Self::NoMatch => "entities-no-match",
            Self::LoadoutHeading => "entities-loadout-heading",
            Self::LoadoutNone => "entities-loadout-none",
            Self::LoadoutResolving => "entities-loadout-resolving",
            Self::LoadoutFailed => "entities-loadout-failed",
            Self::LoadoutStopped => "entities-loadout-stopped",
            Self::LoadoutArchiveChanged => "entities-loadout-archive-changed",
            Self::LoadoutNoPorts => "entities-loadout-no-ports",
            Self::Changes => "entities-changes",
            Self::NoChanges => "entities-no-changes",
            Self::Reset => "entities-reset",
            Self::Export => "entities-export",
            Self::Include => "entities-include",
            Self::Replace => "entities-replace",
            Self::Restore => "entities-restore",
            Self::StatusExcluded => "entities-status-excluded",
            Self::StatusReplaced => "entities-status-replaced",
            Self::StatusDocked => "entities-status-docked",
            Self::StatusEmpty => "entities-status-empty",
            Self::StatusUnresolved => "entities-status-unresolved",
            Self::ExpandPort => "entities-expand-port",
            Self::CollapsePort => "entities-collapse-port",
            Self::ReplaceTitle => "entities-replace-title",
            Self::ReplaceTakes => "entities-replace-takes",
            Self::ReplaceTakesTypes => "entities-replace-takes-types",
            Self::ReplaceTakesSizes => "entities-replace-takes-sizes",
            Self::ReplaceTakesAny => "entities-replace-takes-any",
            Self::ReplaceHint => "entities-replace-hint",
            Self::ReplaceCount => "entities-replace-count",
            Self::ReplaceNone => "entities-replace-none",
            Self::FactsSized => "entities-facts-sized",
            Self::Facts => "entities-facts",
            Self::FactsNone => "entities-facts-none",
        }
    }
}

impl From<EntityText> for super::Text {
    fn from(text: EntityText) -> Self {
        Self::Entity(text)
    }
}
