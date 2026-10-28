// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The ships and items of the playability fixture.

pub(super) const CREWED_MANIFEST: u128 = 0x1100_0000_0000_4000_8000_0000_0000_0001;
pub(super) const EMPTY_MANIFEST: u128 = 0x1100_0000_0000_4000_8000_0000_0000_0002;
pub(super) const UNMANNED_TAG: u128 = 0x1200_0000_0000_4000_8000_0000_0000_0001;
pub(super) const OTHER_TAG: u128 = 0x1200_0000_0000_4000_8000_0000_0000_0002;

/// How one synthetic ship is set up.
#[derive(Clone, Copy)]
pub(super) struct ShipSpec {
    pub(super) class: &'static str,
    pub(super) folder: &'static str,
    pub(super) inclusion: Option<&'static str>,
    pub(super) manifest: Option<u128>,
    pub(super) explosion: f32,
    pub(super) hull_explodes: bool,
    pub(super) tags: &'static [u128],
    pub(super) record_tag: &'static str,
}

const PLAIN: ShipSpec = ShipSpec {
    class: "",
    folder: "spaceships",
    inclusion: Some("ReadyToInclude"),
    manifest: None,
    explosion: 0.0,
    hull_explodes: false,
    tags: &[OTHER_TAG],
    record_tag: "",
};

pub(super) const SHIPS: [ShipSpec; 20] = [
    ShipSpec {
        class: "Rover_Systems_Design",
        folder: "groundvehicles",
        record_tag: "SystemsDesign",
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Player",
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Crewed",
        manifest: Some(CREWED_MANIFEST),
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Explosive",
        explosion: 1.0,
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Hull_Not_Shipped",
        inclusion: Some("DoNotInclude"),
        hull_explodes: true,
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Salvage",
        tags: &[OTHER_TAG, UNMANNED_TAG],
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Not_Shipped",
        inclusion: Some("DoNotInclude"),
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Ready_Explosive_Hull",
        hull_explodes: true,
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_No_Inclusion",
        inclusion: None,
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Empty_Manifest",
        manifest: Some(EMPTY_MANIFEST),
        ..PLAIN
    },
    ShipSpec {
        class: "Rover_Player",
        folder: "groundvehicles",
        explosion: 1.0,
        hull_explodes: true,
        ..PLAIN
    },
    ShipSpec {
        class: "Rover_Salvage",
        folder: "groundvehicles",
        tags: &[UNMANNED_TAG],
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Crewed_Salvage",
        manifest: Some(CREWED_MANIFEST),
        tags: &[UNMANNED_TAG],
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Not_Shipped_Salvage",
        inclusion: Some("DoNotInclude"),
        tags: &[UNMANNED_TAG],
        ..PLAIN
    },
    ShipSpec {
        class: "Rover_Crewed",
        folder: "groundvehicles",
        manifest: Some(CREWED_MANIFEST),
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Other_Inclusion",
        inclusion: Some("SomethingNew"),
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Systems_Design",
        record_tag: "SystemsDesign",
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Systems_Design_Not_Shipped",
        inclusion: Some("DoNotInclude"),
        record_tag: "SystemsDesign",
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Systems_Design_Salvage",
        tags: &[UNMANNED_TAG],
        record_tag: "SystemsDesign",
        ..PLAIN
    },
    ShipSpec {
        class: "Ship_Other_Record_Tag",
        record_tag: "Design",
        ..PLAIN
    },
];

/// How one synthetic item is set up.
#[derive(Clone, Copy)]
pub(super) struct ItemSpec {
    pub(super) class: &'static str,
    pub(super) path: &'static str,
    pub(super) name: Option<&'static str>,
    pub(super) shop: Option<&'static str>,
}

pub(super) const ITEMS: [ItemSpec; 8] = [
    ItemSpec {
        class: "Item_Shop",
        path: "scitem/ships/powerplant/item_shop",
        name: Some("@item_NameShop"),
        shop: Some("@item_NameShop"),
    },
    ItemSpec {
        class: "Item_Shop_Without_Name",
        path: "scitem/ships/powerplant/item_shop_without_name",
        name: None,
        shop: Some("@item_NameShop"),
    },
    ItemSpec {
        class: "Item_Mission",
        path: "scitem/mission_entities/item_mission",
        name: None,
        shop: None,
    },
    ItemSpec {
        class: "Item_Mission_Stash",
        path: "scitem/missionstorage/item_mission_stash",
        name: Some("@item_NameStash"),
        shop: None,
    },
    ItemSpec {
        class: "Item_Nameless",
        path: "scitem/doors/item_nameless",
        name: Some("@LOC_PLACEHOLDER"),
        shop: Some("@LOC_PLACEHOLDER"),
    },
    ItemSpec {
        class: "Item_Named",
        path: "scitem/carryables/item_named",
        name: Some("@item_NameNamed"),
        shop: None,
    },
    ItemSpec {
        class: "Item_Mission_Shop",
        path: "scitem/mission_entities/item_mission_shop",
        name: Some("@item_NameShop"),
        shop: Some("@item_NameShop"),
    },
    ItemSpec {
        class: "Item_Old",
        path: "legacyitems/item_old",
        name: None,
        shop: None,
    },
];
