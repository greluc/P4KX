// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The `itemRecovery` object of the minimal catalog.

use serde_json::json;

pub(super) fn item_recovery() -> serde_json::Value {
    json!({
        "guid": "c4ac0f5a-c072-4666-b3a0-5aef0cf3ff65",
        "className": "GlobalItemRecovery",
        "recordPath": "libs/foundry/records/entitlementpolicies/globalitemrecovery",
        "globalBrickTimer": 10.0,
        "baseReclaimTime": 0.0,
        "baseDeliveryTime": 0.0,
        "scalingPriceFloor": 0.0,
        "aUECperSecond": 1e-6,
        "defaultLoadoutCooldownMultiplier": 200.0,
        "defaultLoadoutDeliveryTimeMultiplier": 200.0,
        "claimCostMultiplier": 0.003,
        "vehicleChassisRepairCostMultiplier": 1.0,
        "vehicleComponentRepairCostMultiplier": 1.0,
        "vehicleRepairMissingItemCostMultiplier": 1.0,
        "deliverySecondsPerAUEC": 1e-6,
        "expeditedDeliverySecondsPerAUEC": 5e-7,
        "defaultLoadoutExpeditedDeliveryTimeMultiplier": 100.0,
        "flatMinimumDeliveryTimerSeconds": 10.0,
        "maxExpeditedClaimCostMultiplier": 0.0003,
        "escalatingClaimsParams": {
            "escalatingClaimChainLimit": 1,
            "escalationTimerMultiplier": 1.0,
            "escalatingClaimCostMultiplier": 1.0,
            "escalatingDeliveryTimerMultiplier": 1.0
        },
        "overrides": [{
            "classGuids": ["90000000-0000-4000-8000-000000000001"],
            "claimCooldownMultiplier": 0.6,
            "claimCostMultiplier": 0.6,
            "deliveryTimeMultiplier": 0.6,
            "maxExpediteCostMultiplier": 0.6,
            "expeditedDeliveryTimeMultiplier": 0.6,
            "flatMinimumDeliveryTimerSecondsMultiplier": 1.0,
            "escalationTimerMultiplier": 1.0,
            "escalatingClaimCostMultiplier": 1.0,
            "escalatingDeliveryTimerMultiplier": 1.0
        }]
    })
}
