use p4kx_cryxml::Xml;

use crate::mtl::{MAX_SUB_MATERIALS, MAX_TEXTURE_SLOTS};
use crate::mtl_xml::XmlElement;
use crate::{Material, MaterialError, MaterialFlags, MaterialLibrary, TextureSlot};

const DEFAULT_DIFFUSE: [f32; 3] = [1.0; 3];
const DEFAULT_SPECULAR: [f32; 3] = [0.04; 3];
const DEFAULT_SHININESS: f32 = 10.0;

pub(crate) fn parse_library(path: &str, bytes: &[u8]) -> Result<MaterialLibrary, MaterialError> {
    match p4kx_cryxml::read(bytes)? {
        Xml::CryXml(document) => library(path, document.root()),
        Xml::Text(text) => {
            let text = String::from_utf8_lossy(text);
            let document = roxmltree::Document::parse(&text)?;
            library(path, document.root_element())
        }
    }
}

fn library<'a>(path: &str, root: impl XmlElement<'a>) -> Result<MaterialLibrary, MaterialError> {
    if !root.name().eq_ignore_ascii_case("Material") {
        return Err(MaterialError::NotMaterial {
            found: root.name().to_owned(),
        });
    }
    let default_palette = root
        .attribute("DefaultPalette")
        .filter(|palette| !palette.is_empty())
        .map(str::to_owned);
    let sub_materials = root
        .children()
        .find(|child| child.name().eq_ignore_ascii_case("SubMaterials"));
    let Some(sub_materials) = sub_materials else {
        return Ok(MaterialLibrary {
            path: path.to_owned(),
            default_palette,
            single: true,
            unknown_entries: 0,
            materials: vec![material(root)?],
        });
    };
    let mut materials = Vec::new();
    let mut unknown_entries = 0;
    for child in sub_materials.children() {
        if materials.len() == MAX_SUB_MATERIALS {
            return Err(MaterialError::TooMany {
                what: "sub-materials",
                limit: MAX_SUB_MATERIALS,
            });
        }
        if child.name().eq_ignore_ascii_case("Material") {
            materials.push(material(child)?);
        } else {
            unknown_entries += 1;
            materials.push(Material::engine_default());
        }
    }
    Ok(MaterialLibrary {
        path: path.to_owned(),
        default_palette,
        single: false,
        unknown_entries,
        materials,
    })
}

fn material<'a>(element: impl XmlElement<'a>) -> Result<Material, MaterialError> {
    let text = |name: &str| element.attribute(name).unwrap_or_default().to_owned();
    let number =
        |name: &str, default: f32| element.attribute(name).and_then(scalar).unwrap_or(default);
    let colour =
        |name: &str, default: [f32; 3]| element.attribute(name).and_then(vector).unwrap_or(default);
    Ok(Material {
        name: text("Name"),
        flags: flags(element.attribute("MtlFlags")),
        shader: text("Shader"),
        surface_type: text("SurfaceType"),
        string_gen_mask: text("StringGenMask"),
        diffuse: colour("Diffuse", DEFAULT_DIFFUSE),
        specular: colour("Specular", DEFAULT_SPECULAR),
        emissive: colour("Emissive", [0.0; 3]),
        shininess: number("Shininess", DEFAULT_SHININESS),
        opacity: number("Opacity", 1.0),
        alpha_test: number("AlphaTest", 0.0),
        glow: number("Glow", 0.0),
        textures: textures(element)?,
    })
}

fn textures<'a>(element: impl XmlElement<'a>) -> Result<Vec<TextureSlot>, MaterialError> {
    let mut slots = Vec::new();
    let lists = element
        .children()
        .filter(|child| child.name().eq_ignore_ascii_case("Textures"));
    for texture in lists
        .flat_map(XmlElement::children)
        .filter(|child| child.name().eq_ignore_ascii_case("Texture"))
    {
        let file = texture.attribute("File").unwrap_or_default();
        if file.is_empty() {
            continue;
        }
        if slots.len() == MAX_TEXTURE_SLOTS {
            return Err(MaterialError::TooMany {
                what: "texture slots in one material",
                limit: MAX_TEXTURE_SLOTS,
            });
        }
        slots.push(TextureSlot {
            map: texture.attribute("Map").unwrap_or_default().to_owned(),
            file: file.to_owned(),
        });
    }
    Ok(slots)
}

/// Reads `MtlFlags`, which files write as a signed 32-bit number.
fn flags(value: Option<&str>) -> MaterialFlags {
    let bits = value
        .and_then(|value| value.trim().parse::<i64>().ok())
        .map_or(0, |value| value & 0xFFFF_FFFF);
    MaterialFlags(u32::try_from(bits).unwrap_or(0))
}

fn scalar(value: &str) -> Option<f32> {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

fn vector(value: &str) -> Option<[f32; 3]> {
    let mut parts = value.split(',').map(scalar);
    let colour = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(colour)
}
