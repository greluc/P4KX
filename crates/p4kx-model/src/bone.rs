/// A rigid transform of a bone: a unit rotation and a translation, without scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoneTransform {
    /// The rotation as a quaternion `x, y, z, w`.
    pub rotation: [f32; 4],
    /// The translation in metres.
    pub translation: [f32; 3],
}

impl BoneTransform {
    /// The transform that changes nothing.
    pub const IDENTITY: Self = Self {
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [0.0; 3],
    };
}

/// One bone of a skeleton, in `CryEngine` coordinates: Z up, metres.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Bone {
    /// The bone's name; may be empty.
    pub name: String,
    /// The index of the parent bone, or `None` for a root. A bone's parent comes before it in
    /// `CompiledBones2` skeletons of the real Archives, but the reader does not rely on it.
    pub parent: Option<usize>,
    /// The identifier animations address the bone by (#107).
    pub controller_id: u32,
    /// The index of the node of the model's `NodeMeshCombo` the bone drives, from the `CompiledBones`
    /// form that static models with animated parts have; `None` for the skeleton of a skin.
    pub node: Option<usize>,
    /// The bind pose relative to the parent bone, or to the model for a root.
    pub relative: BoneTransform,
    /// The bind pose in the model's space.
    pub world: BoneTransform,
}
