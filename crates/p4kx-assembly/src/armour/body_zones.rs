// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::BodyVariant;

/// The zones of the male body skin, indexed by the node number of its submeshes.
const MALE: [&str; 30] = [
    "r_leg01_zone",
    "l_arm05_zone",
    "l_leg01_zone",
    "torso02_zone",
    "r_leg02_zone",
    "underwear_zone",
    "r_arm03_zone",
    "l_arm04_zone",
    "r_hand_zone",
    "r_shoulder_zone",
    "r_arm01_zone",
    "l_leg04_zone",
    "l_shoulder_zone",
    "r_arm04_zone",
    "l_arm02_zone",
    "vneck_zone",
    "r_arm02_zone",
    "r_leg03_zone",
    "l_arm03_zone",
    "l_foot_zone",
    "r_arm05_zone",
    "torso04_zone",
    "torso03_zone",
    "r_leg04_zone",
    "l_leg03_zone",
    "r_foot_zone",
    "l_hand_zone",
    "l_arm01_zone",
    "torso01_zone",
    "l_leg02_zone",
];

/// The zones of the female body skin: those of the male one and the bra, which takes the number
/// 27 and moves the zones after it up by one.
const FEMALE: [&str; 31] = [
    "r_leg01_zone",
    "l_arm05_zone",
    "l_leg01_zone",
    "torso02_zone",
    "r_leg02_zone",
    "underwear_zone",
    "r_arm03_zone",
    "l_arm04_zone",
    "r_hand_zone",
    "r_shoulder_zone",
    "r_arm01_zone",
    "l_leg04_zone",
    "l_shoulder_zone",
    "r_arm04_zone",
    "l_arm02_zone",
    "vneck_zone",
    "r_arm02_zone",
    "r_leg03_zone",
    "l_arm03_zone",
    "l_foot_zone",
    "r_arm05_zone",
    "torso04_zone",
    "torso03_zone",
    "r_leg04_zone",
    "l_leg03_zone",
    "r_foot_zone",
    "l_hand_zone",
    "underwear_top_zone",
    "l_arm01_zone",
    "torso01_zone",
    "l_leg02_zone",
];

impl BodyVariant {
    /// Returns the name of the body zone each node number of the variant's body skin stands for:
    /// the submeshes of the skin are the zones that the `MeshChunk`s of worn clothing name.
    #[must_use]
    pub const fn zone_names(self) -> &'static [&'static str] {
        match self {
            Self::Male => &MALE,
            Self::Female => &FEMALE,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{BodyVariant, FEMALE, MALE};

    #[test]
    fn the_zones_of_a_body_are_distinct_lower_case_names() {
        for variant in BodyVariant::ALL {
            let zones = variant.zone_names();
            let distinct: HashSet<&str> = zones.iter().copied().collect();
            assert_eq!(distinct.len(), zones.len());
            assert!(zones.iter().all(|zone| zone.ends_with("_zone")));
            assert!(zones.iter().all(|zone| zone.to_lowercase() == *zone));
        }
    }

    #[test]
    fn the_female_body_is_the_male_one_with_a_bra() {
        let mut expected = MALE.to_vec();
        expected.insert(27, "underwear_top_zone");
        assert_eq!(FEMALE.to_vec(), expected);
    }
}
