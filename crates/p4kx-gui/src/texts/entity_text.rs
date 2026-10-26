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
    /// The number of variants of a base; takes `count` and `number`.
    Variants,
    /// Lists the variants of a base below it; takes `name`.
    ShowVariants,
    /// Folds the variants of a base away; takes `name`.
    HideVariants,
    /// The accessible name of the row of a base; takes `name` and `variants`.
    BaseRow,
    /// The accessible name of the row of a variant; takes `name`.
    VariantRow,
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
    /// The accessible name of the 3D view; takes `name`, `parts` and `triangles`.
    ViewLabel,
    /// The label of the choice of the level of detail.
    ViewLod,
    /// The label of the choice of the materials.
    ViewMaterials,
    /// The materials drawn as plain colours.
    ViewColours,
    /// The materials drawn with their textures.
    ViewTextures,
    /// Shown while the models are read; takes `count`.
    ViewReading,
    /// Shown while the models are prepared; takes `done` and `total`.
    ViewFlattening,
    /// Shown while the textures are read; takes `done` and `total`.
    ViewTextureProgress,
    /// The choice to show the Interior of the Entity with the hull, and its fact: the parts of
    /// the Interior.
    ViewInterior,
    /// Shown while the Interior is read; takes `done` and `total` containers and `models` of
    /// `of` model files.
    ViewInteriorProgress,
    /// The action that stops building the view.
    ViewStop,
    /// Building the view was stopped.
    ViewStopped,
    /// The action that builds the view again.
    ViewRestart,
    /// No part of the Loadout has a model to show.
    ViewEmpty,
    /// The Archive changed on disk, so the view was not built.
    ViewArchiveChanged,
    /// The models are encrypted and the Archive key is missing or wrong.
    ViewKey,
    /// The models cannot be read; takes `reason`.
    ViewFailed,
    /// The fact: the parts of the Assembly.
    ViewParts,
    /// The fact: the distinct meshes the parts share.
    ViewMeshes,
    /// The fact: the triangles of the parts.
    ViewTriangles,
    /// The action that shows every hidden branch; takes `count`.
    ViewShowAll,
    /// The parts of the Loadout the view could not resolve; takes `count`.
    ViewIssues,
    /// The action that hides a branch in the view.
    Hide,
    /// The action that shows a hidden branch in the view.
    Show,
    /// The port is hidden in the view.
    StatusHidden,
}

impl EntityText {
    /// Every text, for checks of the Fluent files.
    pub(crate) const ALL: [Self; 79] = [
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
        Self::Variants,
        Self::ShowVariants,
        Self::HideVariants,
        Self::BaseRow,
        Self::VariantRow,
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
        Self::ViewLabel,
        Self::ViewLod,
        Self::ViewMaterials,
        Self::ViewColours,
        Self::ViewTextures,
        Self::ViewReading,
        Self::ViewFlattening,
        Self::ViewTextureProgress,
        Self::ViewInterior,
        Self::ViewInteriorProgress,
        Self::ViewStop,
        Self::ViewStopped,
        Self::ViewRestart,
        Self::ViewEmpty,
        Self::ViewArchiveChanged,
        Self::ViewKey,
        Self::ViewFailed,
        Self::ViewParts,
        Self::ViewMeshes,
        Self::ViewTriangles,
        Self::ViewShowAll,
        Self::ViewIssues,
        Self::Hide,
        Self::Show,
        Self::StatusHidden,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    #[expect(clippy::too_many_lines, reason = "one arm per text")]
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
            Self::Variants => "entities-variants",
            Self::ShowVariants => "entities-show-variants",
            Self::HideVariants => "entities-hide-variants",
            Self::BaseRow => "entities-base-row",
            Self::VariantRow => "entities-variant-row",
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
            Self::ViewLabel => "entities-view-label",
            Self::ViewLod => "entities-view-lod",
            Self::ViewMaterials => "entities-view-materials",
            Self::ViewColours => "entities-view-colours",
            Self::ViewTextures => "entities-view-textures",
            Self::ViewReading => "entities-view-reading",
            Self::ViewFlattening => "entities-view-flattening",
            Self::ViewTextureProgress => "entities-view-texture-progress",
            Self::ViewInterior => "entities-view-interior",
            Self::ViewInteriorProgress => "entities-view-interior-progress",
            Self::ViewStop => "entities-view-stop",
            Self::ViewStopped => "entities-view-stopped",
            Self::ViewRestart => "entities-view-restart",
            Self::ViewEmpty => "entities-view-empty",
            Self::ViewArchiveChanged => "entities-view-archive-changed",
            Self::ViewKey => "entities-view-key",
            Self::ViewFailed => "entities-view-failed",
            Self::ViewParts => "entities-view-parts",
            Self::ViewMeshes => "entities-view-meshes",
            Self::ViewTriangles => "entities-view-triangles",
            Self::ViewShowAll => "entities-view-show-all",
            Self::ViewIssues => "entities-view-issues",
            Self::Hide => "entities-hide",
            Self::Show => "entities-show",
            Self::StatusHidden => "entities-status-hidden",
        }
    }
}

impl From<EntityText> for super::Text {
    fn from(text: EntityText) -> Self {
        Self::Entity(text)
    }
}
