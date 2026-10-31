// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::path_match::starts_with_path;

/// A body variant of the human Characters: the skeleton, the default body and the tag that
/// selects the geometry of armour and clothing for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BodyVariant {
    /// The male body (`male_v7`).
    Male,
    /// The female body (`female_v2`).
    Female,
}

impl BodyVariant {
    /// Every body variant.
    pub const ALL: [Self; 2] = [Self::Male, Self::Female];

    /// Returns the stable code of the variant, e.g. `male`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Male => "male",
            Self::Female => "female",
        }
    }

    /// Returns the variant with the code `code`, ignoring ASCII case.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|variant| variant.code().eq_ignore_ascii_case(code))
    }

    /// Returns the tag an Entity Record gives the geometry worn on this body, e.g. `Male`.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Male => "Male",
            Self::Female => "Female",
        }
    }

    /// Returns the skeleton file (`.chr`) of the variant, a path below `Data`.
    #[must_use]
    pub const fn skeleton(self) -> &'static str {
        match self {
            Self::Male => "Objects/Characters/Human/male_v7/export/bhm_skeleton_v7.chr",
            Self::Female => "Objects/Characters/Human/female_v2/export/bhf_skeleton_v2.chr",
        }
    }

    /// Returns the variant whose folder holds the model `path`, a path below `Data`.
    #[must_use]
    pub(crate) fn of_model_path(path: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|variant| starts_with_path(path, variant.folder()))
    }

    /// Returns the folder below `Data` that holds the models of the body, with a trailing
    /// separator.
    #[must_use]
    pub const fn folder(self) -> &'static str {
        match self {
            Self::Male => "Objects/Characters/Human/male_v7/",
            Self::Female => "Objects/Characters/Human/female_v2/",
        }
    }

    /// Returns the skin of the default body, a path below `Data`.
    #[must_use]
    pub const fn skin(self) -> &'static str {
        match self {
            Self::Male => "Objects/Characters/Human/male_v7/body/m_body.skin",
            Self::Female => "Objects/Characters/Human/female_v2/body/f_body.skin",
        }
    }

    /// Returns the folders below `Data` that hold the animations of the body and of the head of
    /// the variant: the databases and animation files its skeleton's parameters
    /// (`bhm_skeleton_v7.chrparams`, `bhf_skeleton_v2.chrparams`) list. Nested folders are
    /// included.
    #[must_use]
    pub const fn animation_folders(self) -> [&'static str; 3] {
        match self {
            Self::Male => [
                "Animations/Characters/Human/male_v7",
                "Animations/pu/male",
                "Animations/Characters/Human/heads/male",
            ],
            Self::Female => [
                "Animations/Characters/Human/female_v2",
                "Animations/pu/female",
                "Animations/Characters/Human/heads/female",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BodyVariant;

    #[test]
    fn the_skeleton_and_the_skin_lie_in_the_folder_of_the_body() {
        for variant in BodyVariant::ALL {
            assert!(
                variant.skeleton().starts_with(variant.folder()),
                "{variant:?}"
            );
            assert!(variant.skin().starts_with(variant.folder()), "{variant:?}");
        }
    }

    #[test]
    fn a_model_in_the_folder_of_a_body_names_that_body() {
        assert_eq!(
            BodyVariant::of_model_path("objects\\characters\\human\\female_v2\\body\\f_body.skin"),
            Some(BodyVariant::Female)
        );
        assert_eq!(
            BodyVariant::of_model_path("Objects/Characters/Human/male_v7/body/m_body.skin"),
            Some(BodyVariant::Male)
        );
        assert_eq!(
            BodyVariant::of_model_path("Objects/Characters/Creatures/x.skin"),
            None
        );
    }

    #[test]
    fn a_code_finds_its_variant_whatever_the_case() {
        for variant in BodyVariant::ALL {
            assert_eq!(BodyVariant::from_code(variant.code()), Some(variant));
        }
        assert_eq!(BodyVariant::from_code("FEMALE"), Some(BodyVariant::Female));
        assert_eq!(BodyVariant::from_code("other"), None);
    }
}
