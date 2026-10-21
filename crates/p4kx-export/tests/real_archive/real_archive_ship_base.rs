//! The base data of ships in a real Archive: mass, hull, cargo, size and default Loadout of every
//! ship and of three known ones; runs with `--ignored` when `P4KX_DATA_P4K` names one.
//!
//! Run with `--nocapture` to see the summary; it holds counts, class names and numbers only.

use std::error::Error;
use std::sync::OnceLock;
use std::time::SystemTime;

use p4kx_export::{
    BuiltShipBase, MassBasis, ShipBase, ShipBaseOptions, SourceKind, build_ship_base,
};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const MIN_SHIPS: usize = 500;
const HARBINGER: &str = "AEGS_Vanguard_Harbinger";
const HARBINGER_MASS: f64 = 230_404.0;
const HARBINGER_HULL: f64 = 48_490.0;
const FREELANCER_MAX: &str = "MISC_Freelancer_MAX";
const FREELANCER_MAX_MASS: f64 = 341_408.0;
const CARRACK: &str = "ANVL_Carrack";
const ANDROMEDA: &str = "RSI_Constellation_Andromeda";

fn ship<'b>(built: &'b BuiltShipBase, class_name: &str) -> Result<&'b ShipBase, Box<dyn Error>> {
    built
        .ships()
        .iter()
        .find(|ship| ship.class_name.eq_ignore_ascii_case(class_name))
        .ok_or_else(|| format!("the Archive has no ship {class_name}").into())
}

#[expect(
    clippy::print_stdout,
    reason = "the counts are the result of the verification"
)]
fn report(built: &BuiltShipBase) {
    let ships = built.ships();
    let with = |test: fn(&ShipBase) -> bool| ships.iter().filter(|ship| test(ship)).count();
    let cleaned: usize = ships
        .iter()
        .filter_map(|ship| ship.mass.as_ref())
        .map(|mass| mass.cleaned)
        .sum();
    let unreadable: usize = ships
        .iter()
        .filter_map(|ship| ship.mass.as_ref())
        .map(|mass| mass.unreadable.len())
        .sum();
    let grid_issues: usize = ships
        .iter()
        .flat_map(|ship| &ship.cargo.grids)
        .filter(|grid| grid.issue.is_some())
        .count();
    println!(
        "ships {} skipped {} failed {}: size {} mass {} (physics {}) hull {} cargo {} loadout {} issues {}",
        ships.len(),
        built.skipped(),
        built.failures().len(),
        with(|s| s.size.is_some()),
        with(|s| s.mass.is_some()),
        with(|s| s
            .mass
            .as_ref()
            .is_some_and(|m| m.basis == MassBasis::Physics)),
        with(|s| s.hull.is_some()),
        with(|s| !s.cargo.grids.is_empty()),
        with(|s| !s.loadout.is_empty()),
        with(|s| !s.issues.is_empty()),
    );
    println!(
        "numbers cleaned of format characters {cleaned}, unreadable mass values {unreadable}, grids with an issue {grid_issues}"
    );
    for class_name in [
        HARBINGER,
        FREELANCER_MAX,
        CARRACK,
        ANDROMEDA,
        "AEGS_Avenger_Titan",
    ] {
        if let Ok(ship) = ship(built, class_name) {
            println!(
                "{class_name}: size {:?} mass {:?} hull {:?} cargo {:?} scu in {} grids, {} loadout ports",
                ship.size.as_ref().map(|s| s.value),
                ship.mass.as_ref().map(|m| m.kg),
                ship.hull.as_ref().map(|h| h.health),
                ship.cargo.scu,
                ship.cargo.grids.len(),
                ship.loadout.len()
            );
        }
    }
}

fn built() -> Result<&'static BuiltShipBase, Box<dyn Error>> {
    static BUILT: OnceLock<Result<BuiltShipBase, String>> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let read = || -> Result<BuiltShipBase, Box<dyn Error>> {
                let archive =
                    Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
                Ok(build_ship_base(
                    &archive,
                    &ShipBaseOptions::new(SystemTime::now()),
                )?)
            };
            read().map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|reason| reason.clone().into())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_ship_is_read_and_most_have_mass_hull_and_size() -> TestResult {
    let built = built()?;
    report(built);
    assert!(built.failures().is_empty(), "{:?}", built.failures());
    assert!(
        built.ships().len() >= MIN_SHIPS,
        "{} ships",
        built.ships().len()
    );
    let with_implementation = built
        .ships()
        .iter()
        .filter(|ship| ship.implementation.is_some())
        .count();
    let with_mass = built
        .ships()
        .iter()
        .filter(|ship| ship.mass.is_some())
        .count();
    assert!(
        with_mass * 10 >= with_implementation * 9,
        "{with_mass} of {with_implementation} ships with an implementation have a mass"
    );
    for ship in built.ships() {
        if let Some(mass) = &ship.mass {
            assert!(mass.kg > 0.0, "{}: mass {}", ship.class_name, mass.kg);
            assert!(
                !mass.sources.is_empty(),
                "{}: mass without a source",
                ship.class_name
            );
        }
        for grid in &ship.cargo.grids {
            assert!(
                grid.scu.is_some() || grid.issue.is_some(),
                "{}: grid {} has neither capacity nor issue",
                ship.class_name,
                grid.port
            );
        }
    }
    let sorted = built
        .ships()
        .windows(2)
        .all(|pair| pair[0].guid.to_string() <= pair[1].guid.to_string());
    assert!(sorted, "ordered by GUID");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_harbinger_is_lighter_after_its_modification() -> TestResult {
    let built = built()?;
    let harbinger = ship(built, HARBINGER)?;
    let mass = harbinger.mass.as_ref().ok_or("the Harbinger has no mass")?;
    assert_eq!(mass.basis, MassBasis::Parts);
    assert!(
        (mass.kg - HARBINGER_MASS).abs() <= 1.0,
        "238,070 kg before the Modification, 230,404 after: {}",
        mass.kg
    );
    assert!(
        mass.sources
            .iter()
            .any(|share| share.source.kind == SourceKind::Modification),
        "the Elem on the root part sets the hull mass: {:?}",
        mass.sources
    );
    let hull = harbinger.hull.as_ref().ok_or("the Harbinger has no hull")?;
    assert!(
        (hull.health - HARBINGER_HULL).abs() <= 1.0,
        "the Harbinger's hull: {}",
        hull.health
    );
    assert!(hull.parts > 0);
    assert_eq!(harbinger.cargo.scu, Some(0.0));
    assert!(harbinger.cargo.scu.is_some_and(f64::is_sign_positive));
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_freelancer_max_has_its_patched_mass_and_about_120_scu() -> TestResult {
    let built = built()?;
    let freelancer = ship(built, FREELANCER_MAX)?;
    let mass = freelancer
        .mass
        .as_ref()
        .ok_or("the Freelancer MAX has no mass")?;
    assert!(
        (mass.kg - FREELANCER_MAX_MASS).abs() <= 1.0,
        "219,396 kg before the Modification, 341,408 after: {}",
        mass.kg
    );
    assert!(
        freelancer
            .cargo
            .scu
            .is_some_and(|scu| (scu - 120.0).abs() <= 0.5),
        "the Freelancer MAX carries 120 SCU: {:?}",
        freelancer.cargo.scu
    );
    assert!(freelancer.size.is_some(), "AttachDef.Size");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_carrack_has_456_scu_and_the_andromeda_does_not_count_its_docked_snub() -> TestResult {
    let built = built()?;
    let carrack = ship(built, CARRACK)?;
    assert!(
        carrack
            .cargo
            .scu
            .is_some_and(|scu| (scu - 456.0).abs() <= 0.5),
        "the Carrack carries 456 SCU: {:?}",
        carrack.cargo.scu
    );
    let andromeda = ship(built, ANDROMEDA)?;
    assert!(
        andromeda
            .cargo
            .scu
            .is_some_and(|scu| (80.0..=110.0).contains(&scu)),
        "the Andromeda carries 96 SCU, without the snub's grids: {:?}",
        andromeda.cargo.scu
    );
    let docked: Vec<_> = andromeda
        .loadout
        .iter()
        .filter(|port| port.docked_vehicle)
        .collect();
    assert!(!docked.is_empty(), "the Andromeda carries a docked vehicle");
    for port in &docked {
        assert!(
            !andromeda
                .loadout
                .iter()
                .any(|other| other.port.starts_with(&format!("{}/", port.port))),
            "{} is not expanded",
            port.port
        );
    }
    Ok(())
}
