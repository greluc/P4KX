// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Material, MaterialLibrary};

const SCALP_SLOT: &str = "TexSlot1";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Body {
    Male,
    Female,
}

impl Body {
    const fn folder(self) -> &'static str {
        match self {
            Self::Male => "male",
            Self::Female => "female",
        }
    }

    const fn prefix(self) -> &'static str {
        match self {
            Self::Male => "m_",
            Self::Female => "f_",
        }
    }
}

fn body_of(path: &str) -> Option<Body> {
    let folders = path
        .rsplit_once(['/', '\\'])
        .map_or("", |(folders, _)| folders);
    folders.rsplit(['/', '\\']).find_map(|part| {
        if part.eq_ignore_ascii_case("male") {
            Some(Body::Male)
        } else if part.eq_ignore_ascii_case("female") {
            Some(Body::Female)
        } else {
            None
        }
    })
}

fn swapped(file: &str, from: Body, to: Body) -> Option<String> {
    let (folders, name) = file.rsplit_once(['/', '\\'])?;
    let folders = folders
        .split(['/', '\\'])
        .map(|part| {
            if part.eq_ignore_ascii_case(from.folder()) {
                to.folder()
            } else {
                part
            }
        })
        .collect::<Vec<_>>();
    let lead = name.get(..from.prefix().len())?;
    if !lead.eq_ignore_ascii_case(from.prefix()) {
        return None;
    }
    let rest = name.get(from.prefix().len()..)?;
    let separator = if file.contains('\\') { '\\' } else { '/' };
    Some(format!(
        "{}{separator}{}{rest}",
        folders.join(&separator.to_string()),
        to.prefix()
    ))
}

/// Returns `material` with its scalp density map of the body of `library`, or `None` when it is
/// no hair cap or its map already belongs to that body.
///
/// A hair cap is drawn over the skin by the density map laid out for the UV of its own body's hair
/// skin. The base file of a hair names the other body's map for its caps (`m_hair_31.mtl` the
/// female one) while its colour variants name the right one, so the body of the folder of the
/// library decides, and the map of the other body is swapped for the one with the same name in
/// the folder of the body.
pub(crate) fn of_the_body(library: &MaterialLibrary, material: &Material) -> Option<Material> {
    if !material.is_hair_cap() {
        return None;
    }
    let body = body_of(&library.path)?;
    let mut corrected = material.clone();
    let mut changed = false;
    for texture in &mut corrected.textures {
        if texture.map != SCALP_SLOT {
            continue;
        }
        let Some(other) = body_of(&texture.file).filter(|other| *other != body) else {
            continue;
        };
        if let Some(file) = swapped(&texture.file, other, body) {
            texture.file = file;
            changed = true;
        }
    }
    changed.then_some(corrected)
}

#[cfg(test)]
mod tests {
    use super::{Body, body_of, swapped};

    #[test]
    fn the_body_is_the_nearest_folder_named_male_or_female() {
        assert!(body_of("Objects/h/sc/male/m_hair_31").is_some_and(|b| b == Body::Male));
        assert!(body_of("Objects\\h\\sc\\Female\\x").is_some_and(|b| b == Body::Female));
        assert!(body_of("Objects/h/sc/male").is_none());
        assert!(body_of("Objects/h/x").is_none());
    }

    #[test]
    fn a_map_is_swapped_to_the_other_bodys_folder_and_prefix() {
        assert_eq!(
            swapped(
                "Objects/h/sc/female/textures/f_hair_31_scalp.tif",
                Body::Female,
                Body::Male
            )
            .as_deref(),
            Some("Objects/h/sc/male/textures/m_hair_31_scalp.tif")
        );
        assert_eq!(
            swapped(
                "Objects/h/female/textures/x_scalp.tif",
                Body::Female,
                Body::Male
            ),
            None
        );
    }
}
