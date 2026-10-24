// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_character::{
    BODY_COLOR, EYE_COLOR, HAIR_DYE_COLOR, HAIR_TINT_COLOR, Material as Choice, SubMaterial,
    lowered_hash, name_hash,
};
use p4kx_model::{Material, MaterialLibrary, MaterialStatus, srgb8_to_linear};

const SKIN_SHADER: &str = "HumanSkin_V2";
const EYE_SHADER: &str = "Eye";
const HAIR_SHADERS: [&str; 2] = ["HairPBR", "Hair"];
const DIFFUSE_SLOT: &str = "TexSlot1";

const BLOND: [f32; 3] = [0.45, 0.28, 0.10];
const BROWN: [f32; 3] = [0.08, 0.045, 0.02];
const BLACK: [f32; 3] = [0.012, 0.010, 0.009];
const AUBURN: [f32; 3] = [0.25, 0.06, 0.02];

/// The material choices a Character made for one piece: colours and numbers that replace the
/// parameters of the piece's shaders.
///
/// What a shader does with them is approximated by one colour per material, because glTF has no
/// skin, iris or hair shader: the skin tone rebases the skin texture, the eye colour tints the
/// iris, and the melanin and dye of a hair give its colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Look {
    choice: Choice,
}

impl Look {
    /// Takes the material choices `choice` of a Character.
    #[must_use]
    pub fn new(choice: &Choice) -> Self {
        Self {
            choice: choice.clone(),
        }
    }

    /// Returns how many textures the choice sets, which the export does not draw.
    #[must_use]
    pub fn texture_choices(&self) -> usize {
        self.choice
            .sub_materials
            .iter()
            .map(|sub| sub.textures.len())
            .sum()
    }

    /// Returns the sub-material choice that `material` takes: the one named by the hash of its
    /// name. A hair material takes the first choice that sets a colour or a number when none is
    /// named so, because the choice of a beard or brow keeps the name of the sub-material of the
    /// style chosen before.
    fn sub_for(&self, material: &Material) -> Option<&SubMaterial> {
        let hash = lowered_hash(&material.name);
        self.choice
            .sub_materials
            .iter()
            .find(|sub| sub.name == hash)
            .or_else(|| {
                is_hair(material)
                    .then(|| {
                        self.choice
                            .sub_materials
                            .iter()
                            .find(|sub| !sub.colors.is_empty() || !sub.floats.is_empty())
                    })
                    .flatten()
            })
    }

    /// Returns `status` with the choices applied to the materials they name, or `None` when the
    /// status is no library or no material is named.
    #[must_use]
    pub fn apply(&self, status: &MaterialStatus) -> Option<MaterialStatus> {
        let library = status.library()?;
        let mut changed = false;
        let dyes = library
            .materials
            .iter()
            .any(|material| is_hair(material) && material.public_param("DyeAmount").is_some());
        let mut materials = library.materials.clone();
        for material in &mut materials {
            if let Some(sub) = self.sub_for(material) {
                changed |= customise(material, sub, dyes);
            }
        }
        changed.then(|| {
            let mut library: MaterialLibrary = library.clone();
            library.materials = materials;
            MaterialStatus::Loaded(library)
        })
    }
}

fn is_hair(material: &Material) -> bool {
    HAIR_SHADERS
        .iter()
        .any(|name| material.shader.eq_ignore_ascii_case(name))
}

fn linear([red, green, blue, _]: [u8; 4]) -> [f32; 3] {
    [red, green, blue].map(srgb8_to_linear)
}

fn colour_of(sub: &SubMaterial, name: u32) -> Option<[f32; 3]> {
    sub.colors
        .iter()
        .find(|colour| colour.name == name)
        .map(|colour| linear(colour.rgba))
}

fn number(sub: &SubMaterial, material: &Material, name: &str) -> f32 {
    let hash = name_hash(name);
    sub.floats
        .iter()
        .find(|param| param.name == hash)
        .map(|param| param.value)
        .or_else(|| material.public_param(name).map(|value| value[0]))
        .filter(|value| value.is_finite())
        .map_or(0.0, |value| value.clamp(0.0, 1.0))
}

fn has_diffuse_texture(material: &Material) -> bool {
    material
        .textures
        .iter()
        .any(|texture| texture.map == DIFFUSE_SLOT)
}

/// Applies `sub` to `material`; `dyes` tells whether the library of a hair takes a dye, which the
/// materials of brows do not: their shader has no dye parameters, so the choice's dye is unused.
fn customise(material: &mut Material, sub: &SubMaterial, dyes: bool) -> bool {
    let shader = material.shader.clone();
    if shader.eq_ignore_ascii_case(SKIN_SHADER) {
        let Some(tone) = colour_of(sub, BODY_COLOR) else {
            return false;
        };
        rebase(material, tone, "SourceAverageColor");
        true
    } else if shader.eq_ignore_ascii_case(EYE_SHADER) {
        let Some(iris) = colour_of(sub, EYE_COLOR) else {
            return false;
        };
        rebase(material, iris, "IrisColor");
        true
    } else if is_hair(material) {
        let dyed = colour_of(sub, HAIR_DYE_COLOR);
        let tint = colour_of(sub, HAIR_TINT_COLOR);
        let melanin = name_hash("BaseMelanin");
        if dyed.is_none() && tint.is_none() && sub.floats.iter().all(|param| param.name != melanin)
        {
            return false;
        }
        let dyed = dyed.filter(|_| dyes);
        material.diffuse = hair_colour(material, sub, dyed, tint);
        true
    } else {
        false
    }
}

/// Moves the diffuse colour of `material` from the colour its shader parameter `source` names to
/// `target`: a material with a diffuse texture is scaled by the ratio, keeping the hue, and one
/// without takes `target`.
fn rebase(material: &mut Material, target: [f32; 3], source: &str) {
    if !has_diffuse_texture(material) {
        material.diffuse = target;
        return;
    }
    let from = material.public_param(source).unwrap_or([1.0; 3]);
    let ratio = [0, 1, 2].map(|channel| {
        let was = from.get(channel).copied().unwrap_or(1.0).max(1e-4);
        target.get(channel).copied().unwrap_or(1.0) / was
    });
    let brightest = ratio.into_iter().fold(1.0_f32, f32::max);
    material.diffuse = ratio.map(|channel| channel / brightest);
}

/// Returns the colour of a hair material: the natural colour of its melanin and redness times the
/// tint, mixed with the dye, if there is one, by the dye amount.
fn hair_colour(
    material: &Material,
    sub: &SubMaterial,
    dyed: Option<[f32; 3]>,
    tint: Option<[f32; 3]>,
) -> [f32; 3] {
    let melanin = number(sub, material, "BaseMelanin");
    let redness = number(sub, material, "BaseMelaninRedness");
    let tint = tint.unwrap_or([1.0; 3]);
    let natural = natural_hair(melanin, redness);
    let natural = [0, 1, 2].map(|channel| {
        natural.get(channel).copied().unwrap_or_default() * tint.get(channel).copied().unwrap_or(1.0)
    });
    dyed.map_or(natural, |dye| {
        lerp(natural, dye, number(sub, material, "DyeAmount"))
    })
}

fn lerp(from: [f32; 3], to: [f32; 3], at: f32) -> [f32; 3] {
    [0, 1, 2].map(|channel| {
        let start = from.get(channel).copied().unwrap_or_default();
        let end = to.get(channel).copied().unwrap_or_default();
        (end - start).mul_add(at, start)
    })
}

/// Returns the colour of hair with `melanin` from 0 (blond) over 0.5 (brown) to 1 (black), moved
/// towards auburn by `redness`.
fn natural_hair(melanin: f32, redness: f32) -> [f32; 3] {
    let base = if melanin < 0.5 {
        lerp(BLOND, BROWN, melanin / 0.5)
    } else {
        lerp(BROWN, BLACK, (melanin - 0.5) / 0.5)
    };
    let red = lerp(base, AUBURN.map(|channel| channel * (1.0 - 0.7 * melanin)), 0.6);
    lerp(base, red, redness)
}

#[cfg(test)]
mod tests {
    use p4kx_character::{ColorParam, FloatParam, Guid, Material as Choice, SubMaterial};
    use p4kx_model::{MaterialLibrary, MaterialStatus};

    use super::{BODY_COLOR, EYE_COLOR, HAIR_DYE_COLOR, Look, lowered_hash, name_hash};

    fn library(xml: &str) -> MaterialStatus {
        MaterialStatus::Loaded(
            MaterialLibrary::parse("test", xml.as_bytes()).expect("a valid library"),
        )
    }

    fn look(name: &str, colors: Vec<ColorParam>, floats: Vec<FloatParam>) -> Look {
        Look::new(&Choice {
            attachment: 1,
            base_material: Guid::default(),
            flags: 0,
            sub_materials: vec![SubMaterial {
                name: lowered_hash(name),
                textures: Vec::new(),
                floats,
                colors,
            }],
        })
    }

    fn colour(name: u32, rgba: [u8; 4]) -> ColorParam {
        ColorParam { name, rgba }
    }

    fn diffuse(status: &MaterialStatus, at: usize) -> [f32; 3] {
        status.library().expect("a library").materials[at].diffuse
    }

    const SKIN: &str = r#"<Material><SubMaterials>
        <Material Name="shader_head" Shader="HumanSkin_V2" Diffuse="1,1,1">
          <Textures><Texture Map="TexSlot1" File="a/head_diff.tif"/></Textures>
          <PublicParams SourceAverageColor="0.2,0.1,0.05"/>
        </Material>
        <Material Name="grey" Shader="HumanSkin_V2" Diffuse="1,1,1"/>
        </SubMaterials></Material>"#;

    #[test]
    fn a_skin_with_a_diffuse_texture_is_rebased_by_the_ratio_of_the_skin_tones() {
        let skin = look(
            "shader_head",
            vec![colour(BODY_COLOR, [255, 255, 255, 255])],
            Vec::new(),
        );

        let status = skin.apply(&library(SKIN)).expect("the material changes");

        let [red, green, blue] = diffuse(&status, 0);
        assert!((red - 0.25).abs() < 1e-5, "{red} keeps the hue");
        assert!((green - 0.5).abs() < 1e-5, "{green} keeps the hue");
        assert!((blue - 1.0).abs() < 1e-5, "{blue} is the brightest channel");
    }

    #[test]
    fn a_skin_without_a_diffuse_texture_takes_the_skin_tone() {
        let skin = look(
            "grey",
            vec![colour(BODY_COLOR, [255, 0, 0, 255])],
            Vec::new(),
        );

        let status = skin.apply(&library(SKIN)).expect("the material changes");

        assert_eq!(diffuse(&status, 1), [1.0, 0.0, 0.0]);
        assert_eq!(diffuse(&status, 0), [1.0; 3], "the other material is as it was");
    }

    #[test]
    fn the_iris_colour_tints_an_eye_and_other_materials_stay() {
        let eyes = library(
            r#"<Material><SubMaterials>
            <Material Name="shader_eyeInner" Shader="Eye" Diffuse="1,1,1"/>
            <Material Name="eyeWet" Shader="Eye" Diffuse="1,1,1"/>
            </SubMaterials></Material>"#,
        );
        let iris = look(
            "shader_eyeInner",
            vec![colour(EYE_COLOR, [0, 0, 255, 255])],
            Vec::new(),
        );

        let status = iris.apply(&eyes).expect("the material changes");

        assert_eq!(diffuse(&status, 0), [0.0, 0.0, 1.0]);
        assert_eq!(diffuse(&status, 1), [1.0; 3]);
    }

    const DYEABLE: &str = r#"<Material><SubMaterials>
        <Material Name="hair_m" Shader="HairPBR" Diffuse="1,1,1">
          <PublicParams DyeAmount="0" BaseMelanin="0.5"/>
        </Material></SubMaterials></Material>"#;
    const PLAIN: &str = r#"<Material><SubMaterials>
        <Material Name="hair_m" Shader="HairPBR" Diffuse="1,1,1">
          <PublicParams BaseMelanin="0.5"/>
        </Material></SubMaterials></Material>"#;

    fn hair(melanin: f32, dye_amount: f32, dye: [u8; 4]) -> [f32; 3] {
        hair_of(DYEABLE, melanin, dye_amount, dye)
    }

    fn hair_of(xml: &str, melanin: f32, dye_amount: f32, dye: [u8; 4]) -> [f32; 3] {
        let cards = library(xml);
        let hair = look(
            "hair_m",
            vec![colour(HAIR_DYE_COLOR, dye)],
            vec![
                FloatParam {
                    name: name_hash("BaseMelanin"),
                    value: melanin,
                },
                FloatParam {
                    name: name_hash("DyeAmount"),
                    value: dye_amount,
                },
            ],
        );
        diffuse(&hair.apply(&cards).expect("the material changes"), 0)
    }

    #[test]
    fn hair_goes_from_blond_over_brown_to_black_with_the_melanin() {
        let (blond, brown, black) = (
            hair(0.0, 0.0, [0; 4]),
            hair(0.5, 0.0, [0; 4]),
            hair(1.0, 0.0, [0; 4]),
        );

        assert!(blond[0] > brown[0] && brown[0] > black[0]);
        assert!(blond[0] > blond[2], "blond is warm");
    }

    #[test]
    fn a_full_dye_replaces_the_natural_colour() {
        let dyed = hair(1.0, 1.0, [0, 0, 255, 255]);

        assert_eq!(dyed, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn a_hair_whose_shader_has_no_dye_ignores_the_dye() {
        let plain = hair_of(PLAIN, 0.5, 1.0, [0, 0, 255, 255]);

        assert_eq!(plain, hair_of(PLAIN, 0.5, 0.0, [0; 4]));
        assert!(plain[2] < 0.5, "not blue");
    }

    #[test]
    fn a_hair_takes_the_first_choice_when_no_sub_material_name_matches() {
        let renamed = look(
            "facial_hair_043",
            vec![colour(HAIR_DYE_COLOR, [0, 255, 0, 255])],
            vec![FloatParam {
                name: name_hash("DyeAmount"),
                value: 1.0,
            }],
        );

        let status = renamed
            .apply(&library(DYEABLE))
            .expect("the hair takes the choice");

        assert_eq!(diffuse(&status, 0), [0.0, 1.0, 0.0]);
    }

    #[test]
    fn a_choice_that_names_no_material_changes_nothing() {
        let skin = look("other", vec![colour(BODY_COLOR, [1, 2, 3, 255])], Vec::new());

        assert!(skin.apply(&library(SKIN)).is_none());
        assert!(skin.apply(&MaterialStatus::Unnamed).is_none());
    }
}
