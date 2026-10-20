use crate::model::{BoneMap, TestPositions};
use crate::{TestModel, TestSkeleton};

const COMPILED_BONES_2: u32 = 0xC201_1111;
const COMPILED_BONES: u32 = 0xC201_973C;

impl TestModel {
    /// Returns the cube as a skin of [`TestSkeleton::two_bones`], as the `.skin` and `.skinm`
    /// files of the Archive store one: no `NodeMeshCombo` chunk, float positions, the skeleton in
    /// a `CompiledBones2` chunk of the `.skin` and a bone-map stream in the `.skinm`. The
    /// vertices of the bottom face and the sides' lower edges belong to the root alone, those
    /// above to the tip alone.
    #[must_use]
    pub fn two_bone_skin() -> Self {
        let cube = Self::cube()
            .with_positions(TestPositions::Float)
            .with_nodes(Vec::new())
            .with_skeleton(&TestSkeleton::two_bones());
        let entries = cube
            .vertices
            .iter()
            .map(|vertex| vec![(u16::from(vertex.position[2] > 2.0), 255)])
            .collect();
        cube.with_bone_map(false, entries)
    }

    /// Adds the bones of `skeleton` to the `.cgf` as a `CompiledBones2` chunk.
    #[must_use]
    pub fn with_skeleton(self, skeleton: &TestSkeleton) -> Self {
        self.with_chunk(COMPILED_BONES_2, skeleton.compiled_bones_2())
    }

    /// Adds the bones of `skeleton` to the `.cgf` as a `CompiledBones` chunk, the form of a static
    /// model with animated parts.
    #[must_use]
    pub fn with_compact_skeleton(self, skeleton: &TestSkeleton) -> Self {
        self.with_chunk(COMPILED_BONES, skeleton.compiled_bones())
    }

    /// Adds a bone-map stream to the mesh with the (bone, weight) pairs of each vertex in stream
    /// order: four pairs per vertex, or eight when `wide`; missing pairs are bone 0 with weight 0.
    #[must_use]
    pub fn with_bone_map(mut self, wide: bool, entries: Vec<Vec<(u16, u8)>>) -> Self {
        let map: BoneMap = (wide, entries);
        self.bone_map = Some(map);
        self
    }
}
