// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::{Kind, is_kind_leftover, is_leftover};

#[test]
fn the_leftover_like_ship_names_are_excluded_as_whole_words_only() {
    let ship = |path: &str, name: &str| is_kind_leftover(Kind::Ships, path, name);
    for path in [
        "libs/foundry/records/entities/spaceships/glsn_shiv_temp_cosa",
        "libs/foundry/records/entities/spaceships/aegs_reclaimer_bis2024_temp",
        "libs/foundry/records/entities/spaceships/aegs_avenger_stalker_display",
        "libs/foundry/records/entities/spaceships/orig_890jump_hijacked_pisces",
        "libs/foundry/records/entities/spaceships/drak_caterpillar_pu_hijacked",
    ] {
        assert!(ship(path, "Ship"), "{path}");
    }
    assert!(ship(
        "libs/foundry/records/x",
        "RSI_Meteor_TEMP_LaserRepeater"
    ));
    for path in [
        "libs/foundry/records/entities/spaceships/rsi_temperature_probe",
        "libs/foundry/records/entities/scitem/displays/panel",
        "libs/foundry/records/entities/spaceships/contemporary",
        "libs/foundry/records/entities/spaceships/drak_hijacker",
    ] {
        assert!(!ship(path, "Ship"), "{path}");
    }
    assert!(
        !is_kind_leftover(
            Kind::Items,
            "libs/foundry/records/entities/scitem/ships/displays/radar_display_1m",
            "radar_display_1m"
        ),
        "an item keeps a display word"
    );
}

#[test]
fn a_token_is_a_whole_word_of_the_path_or_the_name() {
    for path in [
        "libs/foundry/records/entities/scitem/weapons/fps_weapons/dev/vlk_spewgun",
        "libs/foundry/records/entities/scitem/usables/counter_bar_1m_deprecated",
        "libs/foundry/records/entities/scitem/carryables/1h/chip_01_debug",
    ] {
        assert!(is_leftover(path, "Name"), "{path}");
    }
    assert!(is_leftover("libs/foundry/records/x", "Chip_DEBUG"), "name");
    for path in [
        "libs/foundry/records/entities/scitem/devices/scanner",
        "libs/foundry/records/entities/scitem/fps_devices/behr_ltp_01",
        "libs/foundry/records/entities/scitem/undeveloped",
        "libs/foundry/records/entities/spaceships/devastator",
    ] {
        assert!(!is_leftover(path, "Scanner"), "{path}");
    }
    assert!(
        !is_leftover("libs/foundry/records/entities/spaceships/x", "Devastator"),
        "a name that starts with the token"
    );
}
