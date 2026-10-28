// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::selection::{Kind, KindRule};

impl Kind {
    /// Returns the rule of a kind that selects Records; a derived kind has the empty rule, which
    /// matches no path.
    #[expect(clippy::too_many_lines, reason = "a table with one entry per kind")]
    pub(crate) const fn records(self) -> KindRule {
        match self {
            Self::Manufacturers => KindRule {
                prefixes: &["libs/foundry/records/scitemmanufacturer/"],
                type_name: "SCItemManufacturer",
            },
            Self::Items => KindRule {
                prefixes: &[
                    "libs/foundry/records/entities/scitem/",
                    "libs/foundry/records/entities/legacyitems/",
                ],
                type_name: "EntityClassDefinition",
            },
            Self::Ships => KindRule {
                prefixes: &[
                    "libs/foundry/records/entities/spaceships/",
                    "libs/foundry/records/entities/groundvehicles/",
                ],
                type_name: "EntityClassDefinition",
            },
            Self::Commodities => KindRule {
                prefixes: &["libs/foundry/records/entities/commodities/"],
                type_name: "EntityClassDefinition",
            },
            Self::Blueprints => KindRule {
                prefixes: &["libs/foundry/records/crafting/blueprints/crafting/"],
                type_name: "CraftingBlueprintRecord",
            },
            Self::ResourceGroups => KindRule {
                prefixes: &["libs/foundry/records/resourcetypedatabase/"],
                type_name: "ResourceTypeGroup",
            },
            Self::BlueprintCategories => KindRule {
                prefixes: &["libs/foundry/records/crafting/blueprintcategories/"],
                type_name: "BlueprintCategoryRecord",
            },
            Self::GameplayProperties => KindRule {
                prefixes: &["libs/foundry/records/crafting/craftedproperties/"],
                type_name: "CraftingGameplayPropertyDef",
            },
            Self::QualityDistributions => KindRule {
                prefixes: &["libs/foundry/records/crafting/qualitydistribution/"],
                type_name: "CraftingQualityDistributionRecord",
            },
            Self::QualityQuantizations => KindRule {
                prefixes: &["libs/foundry/records/crafting/qualityquantization/"],
                type_name: "CraftingQualityQuantizationRecord",
            },
            Self::BlueprintPools => KindRule {
                prefixes: &["libs/foundry/records/crafting/blueprintrewards/"],
                type_name: "BlueprintPoolRecord",
            },
            Self::DismantleProcesses => KindRule {
                prefixes: &["libs/foundry/records/crafting/blueprints/dismantle/"],
                type_name: "CraftingBlueprintRecord",
            },
            Self::StarmapObjects => KindRule {
                prefixes: &["libs/foundry/records/starmap/pu/"],
                type_name: "StarMapObject",
            },
            Self::Contracts => KindRule {
                prefixes: &["libs/foundry/records/contracts/contractgenerator/"],
                type_name: "ContractGenerator",
            },
            Self::MineableElements => KindRule {
                prefixes: &["libs/foundry/records/mining/mineableelements/"],
                type_name: "MineableElement",
            },
            Self::RefiningProcesses => KindRule {
                prefixes: &["libs/foundry/records/refiningprocess/"],
                type_name: "RefiningProcess",
            },
            Self::MineableCompositions => KindRule {
                prefixes: &["libs/foundry/records/mining/rockcompositionpresets/"],
                type_name: "MineableComposition",
            },
            Self::MiningGlobalParams => KindRule {
                prefixes: &["libs/foundry/records/mining/miningglobalparams"],
                type_name: "MiningGlobalParams",
            },
            Self::HarvestableProviderPresets => KindRule {
                prefixes: &["libs/foundry/records/harvestable/providerpresets/"],
                type_name: "HarvestableProviderPreset",
            },
            Self::SubHarvestableMultiConfigs => KindRule {
                prefixes: &["libs/foundry/records/harvestable/slotpresets/"],
                type_name: "SubHarvestableMultiConfigRecord",
            },
            Self::FactionReputations => KindRule {
                prefixes: &["libs/foundry/records/factions/factionreputation/"],
                type_name: "FactionReputation",
            },
            Self::ReputationScopes => KindRule {
                prefixes: &["libs/foundry/records/reputation/scopes/"],
                type_name: "SReputationScopeParams",
            },
            Self::ReputationStandings => KindRule {
                prefixes: &["libs/foundry/records/reputation/standings/"],
                type_name: "SReputationStandingParams",
            },
            Self::ReputationContexts => KindRule {
                prefixes: &["libs/foundry/records/reputation/contexts/"],
                type_name: "SReputationContextUI",
            },
            Self::MissionTypes => KindRule {
                prefixes: &["libs/foundry/records/missiontype/"],
                type_name: "MissionType",
            },
            Self::MissionLocalities => KindRule {
                prefixes: &["libs/foundry/records/missiondata/pu_missionlocality/"],
                type_name: "MissionLocality",
            },
            Self::ContractScenarios => KindRule {
                prefixes: &["libs/foundry/records/contracts/contractscenarios/"],
                type_name: "ScenarioProgress",
            },
            Self::LootTables => KindRule {
                prefixes: &["libs/foundry/records/lootgeneration/loottables/"],
                type_name: "LootTableV3Record",
            },
            Self::LootArchetypes => KindRule {
                prefixes: &["libs/foundry/records/lootgeneration/lootarchetypes/"],
                type_name: "LootArchetypeV3Record",
            },
            Self::LootPoolFilters => KindRule {
                prefixes: &["libs/foundry/records/lootgeneration/filters/"],
                type_name: "PoolFilterRecord",
            },
            Self::Paints
            | Self::Loadouts
            | Self::HarvestableLocations
            | Self::Shops
            | Self::LootContainers => KindRule {
                prefixes: &[],
                type_name: "",
            },
        }
    }
}
