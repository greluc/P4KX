// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::*;

const MATRIX: &str = r#"{"success":1,"data":[
    {"id":"1","name":"Alpha","production_status":"flight-ready","url":"/pledge/ships/m/Alpha-One"},
    {"id":2,"name":"Beta","production_status":"in-concept","url":"/pledge/ships/m/Beta"},
    {"id":"3","name":"Gamma","production_status":"flight-ready","url":"/pledge/ships/m/Gamma"},
    {"id":"9","name":"Delta","production_status":"flight-ready","url":"/pledge/ships/m/Delta/"},
    {"id":"x","name":"Epsilon","production_status":"flight-ready","url":"/pledge/ships/m/Eps"}
]}"#;

fn ship<'a>(class_name: &'a str, playability: Option<&'a str>) -> CatalogShip<'a> {
    CatalogShip {
        class_name,
        playability,
    }
}

fn slug<'a>(slug: &'a str, rsi_id: u32, class_name: Option<&'a str>) -> StoreSlug<'a> {
    StoreSlug {
        slug,
        rsi_id,
        class_name,
    }
}

#[test]
fn parses_the_envelope_and_a_bare_list() -> Result<(), serde_json::Error> {
    assert_eq!(parse_matrix(MATRIX)?.len(), 5);
    let bare = parse_matrix(r#"[{"id":1,"name":"A"}]"#)?;
    assert_eq!((bare[0].rsi_id(), bare[0].status()), (Some(1), "no status"));
    Ok(())
}

#[test]
fn reports_each_kind_of_disagreement() -> Result<(), serde_json::Error> {
    let matrix = parse_matrix(MATRIX)?;
    let slugs = [
        slug("Alpha-One", 1, Some("A_One")),
        slug("Beta", 2, Some("B")),
        slug("Gamma", 30, Some("G")),
        slug("Eps", 5, None),
    ];
    let ships = [
        ship("A_One", Some("ai")),
        ship("B", Some("player")),
        ship("G", None),
        ship("Z", Some("player")),
        ship("Y", Some("special")),
    ];
    let report = compare(&ships, &slugs, &matrix);
    let found = |class: &str, playability: &str, matrix: &str| Finding {
        class_name: class.into(),
        playability: playability.into(),
        matrix: matrix.into(),
    };
    assert_eq!(
        report.flight_ready_not_player,
        [found("A_One", "ai", "Alpha (flight-ready)")]
    );
    assert_eq!(
        report.player_not_flight_ready,
        [
            found("B", "player", "Beta (in-concept)"),
            found("Z", "player", UNKNOWN)
        ]
    );
    assert_eq!(
        report.null_ships,
        [found("G", "null", "Gamma (flight-ready)")]
    );
    assert_eq!(report.flight_ready_unmapped, ["Delta", "Epsilon"]);
    assert!(report.lines().iter().any(|line| line.contains("G [null]")));
    Ok(())
}

#[test]
fn a_slug_that_differs_only_in_case_maps() -> Result<(), serde_json::Error> {
    let matrix = parse_matrix(MATRIX)?;
    let report = compare(
        &[ship("A_One", None)],
        &[slug("aLpHa-oNe", 100, Some("A_One"))],
        &matrix,
    );
    assert_eq!(report.null_ships[0].matrix, "Alpha (flight-ready)");
    Ok(())
}

#[test]
fn a_trailing_slash_in_the_url_is_ignored() -> Result<(), serde_json::Error> {
    let matrix = parse_matrix(MATRIX)?;
    let delta = matrix.iter().find(|entry| entry.name == "Delta");
    assert_eq!(delta.and_then(MatrixShip::slug), Some("Delta"));
    let report = compare(
        &[ship("D", None)],
        &[slug("Delta", 100, Some("D"))],
        &matrix,
    );
    assert_eq!(report.null_ships[0].matrix, "Delta (flight-ready)");
    Ok(())
}

#[test]
fn malformed_text_is_an_error() {
    for text in ["", "not json", r#"{"data": 3}"#, r#"[{"id":1}]"#] {
        assert!(parse_matrix(text).is_err(), "{text:?}");
    }
}

#[test]
fn an_entry_without_an_id_does_not_fail_the_parse() -> Result<(), serde_json::Error> {
    let matrix = parse_matrix(r#"[{"name":"A"},{"id":null,"name":"B"}]"#)?;
    assert_eq!(matrix.len(), 2);
    assert!(matrix.iter().all(|entry| entry.rsi_id().is_none()));
    Ok(())
}

#[test]
fn a_ready_variant_hides_a_base_entry_that_is_not_ready() -> Result<(), serde_json::Error> {
    let matrix = parse_matrix(
        r#"[{"id":1,"name":"Base","production_status":"in-concept"},
            {"id":2,"name":"Base Edition","production_status":"flight-ready"}]"#,
    )?;
    let slugs = [slug("a", 1, Some("B")), slug("b", 2, Some("B"))];
    let report = compare(&[ship("B", Some("player"))], &slugs, &matrix);
    assert_eq!(report.player_not_flight_ready, []);
    Ok(())
}

#[test]
fn a_supplement_class_missing_from_the_catalog_is_reported() {
    let ships = [ship("ARGO_MOLE", Some("player"))];
    let gaps = supplement_gaps(&ships);
    assert!(!gaps.contains(&"ARGO_MOLE"));
    assert!(gaps.contains(&"GLSN_Basher"));
    let none = supplement_gaps(&[]);
    assert_eq!(
        none.iter().filter(|class| **class == "ARGO_MOLE").count(),
        1
    );
}

#[test]
fn a_supplemented_name_maps_without_a_store_slug() -> Result<(), serde_json::Error> {
    let matrix =
        parse_matrix(r#"[{"id":"77","name":"Skylark","production_status":"flight-ready"}]"#)?;
    let report = compare(&[ship("RSI_Skylark", None)], &[], &matrix);
    assert_eq!(report.null_ships[0].matrix, "Skylark (flight-ready)");
    assert_eq!(report.flight_ready_unmapped, Vec::<String>::new());
    Ok(())
}
