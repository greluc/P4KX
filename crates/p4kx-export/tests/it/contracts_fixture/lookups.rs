use p4kx_testkit::{DataForgeBuilder, TestRecord, TestValue};

use super::{
    AFFINITY, BROKER, DELIVERY, FACTION, FACTION_SCOPE, LOC_A, LOC_B, LOSS, OBJ_PLANET, OBJ_STAR,
    OBJ_STATION, OBJ_SYSTEM, RANK_FIFTY, RANK_TWO_HUNDRED, TYPE_PLANET, TYPE_STAR, TYPE_STATION,
    TYPE_SYSTEM,
};

pub(super) fn plain_record(guid: u128, kind: &str, path: &str) -> TestRecord {
    let name = path.rsplit('/').next().unwrap_or(path);
    TestRecord::new(
        guid,
        kind,
        format!("{kind}.{name}"),
        format!("libs/foundry/records/{path}.xml"),
    )
}

pub(super) fn mission_type(guid: u128, name: &str, key: &str) -> TestRecord {
    TestRecord::new(
        guid,
        "MissionType",
        format!("MissionType.{name}"),
        format!("libs/foundry/records/missiontype/{name}.xml"),
    )
    .value("LocalisedTypeName", TestValue::text(key))
}

fn reward(guid: u128, name: &str, value: i32) -> TestRecord {
    plain_record(
        guid,
        "ReputationReward",
        &format!("reputation/rewards/missionrewards_reputation/{name}"),
    )
    .value("reputationAmount", TestValue::Int32(value))
}

fn object(guid: u128, name: &str, kind: u128) -> TestRecord {
    plain_record(guid, "StarMapObject", &format!("starmap/pu/{name}"))
        .value("name", TestValue::text(format!("@place_{name}")))
        .value("type", TestValue::Reference(kind))
}

fn kind(guid: u128, name: &str) -> TestRecord {
    plain_record(guid, "StarMapObjectType", &format!("starmap/type/{name}"))
        .value("name", TestValue::text(name))
}

fn mission_locality(guid: u128, name: &str, locations: &[u128]) -> TestRecord {
    plain_record(
        guid,
        "MissionLocality",
        &format!("missiondata/pu_missionlocality/{name}"),
    )
    .value(
        "availableLocations",
        TestValue::Array(
            locations
                .iter()
                .copied()
                .map(TestValue::Reference)
                .collect(),
        ),
    )
}

pub(super) fn places(builder: DataForgeBuilder) -> DataForgeBuilder {
    let stations: Vec<u128> = (0..7).map(|offset| OBJ_STATION + offset).collect();
    let mut records = vec![
        kind(TYPE_SYSTEM, "SolarSystem"),
        kind(TYPE_PLANET, "Planet"),
        kind(TYPE_STAR, "Star"),
        kind(TYPE_STATION, "Station"),
        object(OBJ_SYSTEM, "stanton", TYPE_SYSTEM),
        object(OBJ_PLANET, "hurston", TYPE_PLANET),
        object(OBJ_STAR, "sol", TYPE_STAR),
        mission_locality(
            LOC_A,
            "stanton_all",
            &[OBJ_STATION, OBJ_STAR, OBJ_PLANET, OBJ_SYSTEM],
        ),
        mission_locality(LOC_B, "many_stations", &stations),
    ];
    records.extend(
        stations
            .iter()
            .enumerate()
            .map(|(number, guid)| object(*guid, &format!("station{number}"), TYPE_STATION)),
    );
    records.into_iter().fold(builder, DataForgeBuilder::record)
}

pub(super) fn reputation(builder: DataForgeBuilder) -> DataForgeBuilder {
    [
        plain_record(
            FACTION,
            "FactionReputation",
            "factions/factionreputation/acme",
        )
        .value("displayName", TestValue::text("@faction_acme")),
        plain_record(BROKER, "MissionBrokerEntry", "missionbroker/haul")
            .value("title", TestValue::text("@title_broker"))
            .value("type", TestValue::Reference(DELIVERY))
            .value("locationMissionAvailable", TestValue::Reference(OBJ_PLANET)),
        plain_record(
            FACTION_SCOPE,
            "ReputationScope",
            "reputation/scopes/faction",
        ),
        plain_record(AFFINITY, "ReputationScope", "reputation/scopes/affinity"),
        reward(RANK_TWO_HUNDRED, "gain_200", 200),
        reward(RANK_FIFTY, "gain_50", 50),
        reward(LOSS, "loss_100", -100),
    ]
    .into_iter()
    .fold(builder, DataForgeBuilder::record)
}
