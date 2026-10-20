use crate::MaterialError;
use crate::mtl_parse::parse_library;

/// The most sub-materials a material library may hold; the engine allows 128.
pub const MAX_SUB_MATERIALS: usize = 1024;

/// The most texture slots one material may hold.
pub const MAX_TEXTURE_SLOTS: usize = 64;

/// The `MtlFlags` of a material, as the engine's `EMaterialFlags` define them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MaterialFlags(pub u32);

impl MaterialFlags {
    const TWO_SIDED: u32 = 0x0002;
    const NO_DRAW: u32 = 0x0400;

    /// Returns whether the engine draws both faces of a triangle.
    #[must_use]
    pub const fn two_sided(self) -> bool {
        self.0 & Self::TWO_SIDED != 0
    }

    /// Returns whether the engine draws nothing with the material: collision, shield and
    /// helper geometry.
    #[must_use]
    pub const fn no_draw(self) -> bool {
        self.0 & Self::NO_DRAW != 0
    }
}

/// One texture slot of a material.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextureSlot {
    /// The slot, as the file names it: `TexSlot1`, `TexSlot2`, and so on.
    pub map: String,
    /// The texture's path as the file gives it, below `Data`, or an engine placeholder such as
    /// `$RenderToTexture`.
    pub file: String,
}

/// One material: the surface a submesh is drawn with.
///
/// Colours are linear, as the file stores them.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Material {
    /// The material's name; empty when the file gives none.
    pub name: String,
    /// The engine flags.
    pub flags: MaterialFlags,
    /// The shader's name, e.g. `Illum` or `NoDraw`; empty when the file gives none.
    pub shader: String,
    /// The physical surface type, e.g. `metal_dense`; empty when the file gives none.
    pub surface_type: String,
    /// The shader generation mask, e.g. `%NORMAL_MAP%SPECULAR_MAP`; empty when the file gives
    /// none.
    pub string_gen_mask: String,
    /// The diffuse colour.
    pub diffuse: [f32; 3],
    /// The specular colour.
    pub specular: [f32; 3],
    /// The emissive colour, drawn when [`Self::glow`] is above zero.
    pub emissive: [f32; 3],
    /// The glossiness from 0 to 255.
    pub shininess: f32,
    /// The opacity from 0 to 1.
    pub opacity: f32,
    /// The alpha-test threshold, 0 when the material is not alpha tested.
    pub alpha_test: f32,
    /// The glow amount the emissive colour is multiplied by.
    pub glow: f32,
    /// The texture slots in file order.
    pub textures: Vec<TextureSlot>,
}

impl Material {
    /// Returns the material the engine draws a material ID without a sub-material with: a
    /// light grey, opaque, matt surface.
    #[must_use]
    pub fn engine_default() -> Self {
        Self {
            name: String::new(),
            flags: MaterialFlags::default(),
            shader: String::new(),
            surface_type: String::new(),
            string_gen_mask: String::new(),
            diffuse: [0.8; 3],
            specular: [0.04; 3],
            emissive: [0.0; 3],
            shininess: 10.0,
            opacity: 1.0,
            alpha_test: 0.0,
            glow: 0.0,
            textures: Vec::new(),
        }
    }

    /// Returns whether the engine draws nothing with the material: its flags or shader say so,
    /// or it is fully transparent.
    #[must_use]
    pub fn hidden(&self) -> bool {
        self.flags.no_draw() || self.shader.eq_ignore_ascii_case("NoDraw") || self.opacity <= 0.0
    }
}

/// The materials of one `.mtl` file.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MaterialLibrary {
    /// The path the model gave, below `Data` without `.mtl`.
    pub path: String,
    /// The tint palette the library names as its default, a Record path.
    pub default_palette: Option<String>,
    /// Whether the file is one material that serves every material ID, rather than a list of
    /// sub-materials.
    pub single: bool,
    /// The sub-materials in material ID order; one material when [`Self::single`].
    pub materials: Vec<Material>,
}

impl MaterialLibrary {
    /// Reads the `.mtl` file `bytes`, text XML or `CryXml`, of the library `path`.
    ///
    /// A file with a `SubMaterials` element is a list: the material ID is the position of the
    /// sub-material. A file without is one material that serves every ID. Numbers that are
    /// missing, malformed or not finite read as white diffuse, a dim dielectric specular, no
    /// emission, full opacity and a shininess of 10.
    ///
    /// # Errors
    ///
    /// Returns [`MaterialError::Xml`] when the file is no XML or `CryXml`,
    /// [`MaterialError::NotMaterial`] when its root is no `Material`, and
    /// [`MaterialError::TooMany`] when it holds more sub-materials or texture slots than the
    /// limits allow.
    pub fn parse(path: &str, bytes: &[u8]) -> Result<Self, MaterialError> {
        parse_library(path, bytes)
    }

    /// Returns the material of `id`, or `None` when the library has no sub-material at that
    /// position and the engine falls back to its default material.
    #[must_use]
    pub fn material(&self, id: u16) -> Option<&Material> {
        if self.single {
            self.materials.first()
        } else {
            self.materials.get(usize::from(id))
        }
    }
}

/// What reading a model's material library came to.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum MaterialStatus {
    /// The model names no material library.
    Unnamed,
    /// The library the model names does not exist.
    Missing {
        /// The path the model gave.
        path: String,
    },
    /// The library exists but cannot be read, or its name is no safe path.
    Unreadable {
        /// The path the model gave.
        path: String,
        /// What is wrong.
        reason: String,
    },
    /// The library was read.
    Loaded(MaterialLibrary),
}

impl MaterialStatus {
    /// Returns the library when it was read.
    #[must_use]
    pub const fn library(&self) -> Option<&MaterialLibrary> {
        match self {
            Self::Loaded(library) => Some(library),
            _ => None,
        }
    }

    /// Returns what went wrong with the library, or `None` when it was read or the model names
    /// none.
    #[must_use]
    pub fn problem(&self) -> Option<String> {
        match self {
            Self::Missing { path } => Some(format!("the material library {path} does not exist")),
            Self::Unreadable { path, reason } => Some(format!(
                "the material library {path} cannot be read: {reason}"
            )),
            Self::Unnamed | Self::Loaded(_) => None,
        }
    }
}
