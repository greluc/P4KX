// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::mtl::Material;

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

    fn of_folder(part: &str) -> Option<Self> {
        [Self::Male, Self::Female]
            .into_iter()
            .find(|body| part.eq_ignore_ascii_case(body.folder()))
    }
}

fn body_of(path: &str) -> Option<Body> {
    let (folders, _) = path.rsplit_once(['/', '\\'])?;
    folders.rsplit(['/', '\\']).find_map(Body::of_folder)
}

fn swapped(file: &str, from: Body, to: Body) -> Option<String> {
    let (folders, name) = file.rsplit_once(['/', '\\'])?;
    let separator = if file.contains('\\') { '\\' } else { '/' };
    let mut parts: Vec<&str> = folders.split(['/', '\\']).collect();
    let at = parts
        .iter()
        .rposition(|part| Body::of_folder(part) == Some(from))?;
    *parts.get_mut(at)? = to.folder();
    let lead = name.get(..from.prefix().len())?;
    if !lead.eq_ignore_ascii_case(from.prefix()) {
        return None;
    }
    let rest = name.get(from.prefix().len()..)?;
    Some(format!(
        "{}{separator}{}{rest}",
        parts.join(&separator.to_string()),
        to.prefix()
    ))
}

impl Material {
    /// Returns the hair cap with its scalp map of the body of the library at `library_path`, or
    /// `None` when this is no cap, the path names no body or the map is already that body's.
    ///
    /// A heuristic: the `male` or `female` folder of the library is taken as the body, and a map
    /// of the other body's folder and `m_` / `f_` prefix is swapped for the same name in this
    /// body's (ADR-0213).
    #[must_use]
    pub fn hair_cap_of_body(&self, library_path: &str) -> Option<Self> {
        if !self.is_hair_cap() {
            return None;
        }
        let body = body_of(library_path)?;
        let mut corrected = self.clone();
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

    #[test]
    fn only_the_nearest_body_folder_is_swapped() {
        assert_eq!(
            swapped(
                "Objects/female/sc/female/textures/f_a_scalp.tif",
                Body::Female,
                Body::Male
            )
            .as_deref(),
            Some("Objects/female/sc/male/textures/m_a_scalp.tif")
        );
    }
}
