/// The known chunk types of the `#ivo` container.
pub mod ivo {
    use crate::{ChunkType, Container};

    /// The `DbaData` chunk type.
    pub const DBA_DATA: ChunkType = ChunkType::new(Container::Ivo, 0x194F_BC50);
    /// The `BoundingData` chunk type.
    pub const BOUNDING_DATA: ChunkType = ChunkType::new(Container::Ivo, 0x2B7E_CF9F);
    /// The `AnimInfo` chunk type.
    pub const ANIM_INFO: ChunkType = ChunkType::new(Container::Ivo, 0x4733_C6ED);
    /// The `BShapesGpu` chunk type.
    pub const BSHAPES_GPU: ChunkType = ChunkType::new(Container::Ivo, 0x57A3_BEFD);
    /// The `LodMeshData` chunk type.
    pub const LOD_MESH_DATA: ChunkType = ChunkType::new(Container::Ivo, 0x58DE_1772);
    /// The `NodeMeshCombo` chunk type.
    pub const NODE_MESH_COMBO: ChunkType = ChunkType::new(Container::Ivo, 0x7069_7FDA);
    /// The `MtlName320` chunk type.
    pub const MTL_NAME_320: ChunkType = ChunkType::new(Container::Ivo, 0x8335_3333);
    /// The `MtlNameVariant` chunk type.
    pub const MTL_NAME_VARIANT: ChunkType = ChunkType::new(Container::Ivo, 0x8335_3533);
    /// The `MtlName` chunk type.
    pub const MTL_NAME: ChunkType = ChunkType::new(Container::Ivo, 0x8335_674E);
    /// The `BShapes` chunk type.
    pub const BSHAPES: ChunkType = ChunkType::new(Container::Ivo, 0x875C_CB28);
    /// The `CompiledPhysicalBones320` chunk type.
    pub const COMPILED_PHYSICAL_BONES_320: ChunkType = ChunkType::new(Container::Ivo, 0x90C6_6666);
    /// The `CompiledPhysicalBones` chunk type.
    pub const COMPILED_PHYSICAL_BONES: ChunkType = ChunkType::new(Container::Ivo, 0x90C6_87DC);
    /// The `MeshInfo` chunk type.
    pub const MESH_INFO: ChunkType = ChunkType::new(Container::Ivo, 0x9291_4444);
    /// The `Mesh` chunk type.
    pub const MESH: ChunkType = ChunkType::new(Container::Ivo, 0x9293_B9D8);
    /// The `LodDistances` chunk type.
    pub const LOD_DISTANCES: ChunkType = ChunkType::new(Container::Ivo, 0x9351_756F);
    /// The `CafData` chunk type.
    pub const CAF_DATA: ChunkType = ChunkType::new(Container::Ivo, 0xA949_6CB5);
    /// The `Skin2` chunk type.
    pub const SKIN_2: ChunkType = ChunkType::new(Container::Ivo, 0xB875_7777);
    /// The `Skin` chunk type.
    pub const SKIN: ChunkType = ChunkType::new(Container::Ivo, 0xB875_B2D9);
    /// The `AssetMetadata` chunk type.
    pub const ASSET_METADATA: ChunkType = ChunkType::new(Container::Ivo, 0xBE5E_493E);
    /// The `CompiledBones2` chunk type.
    pub const COMPILED_BONES_2: ChunkType = ChunkType::new(Container::Ivo, 0xC201_1111);
    /// The `CompiledBones` chunk type.
    pub const COMPILED_BONES: ChunkType = ChunkType::new(Container::Ivo, 0xC201_973C);
    /// The `Terminator` chunk type.
    pub const TERMINATOR: ChunkType = ChunkType::new(Container::Ivo, 0xE018_1074);
    /// The `DbaMetadata` chunk type.
    pub const DBA_METADATA: ChunkType = ChunkType::new(Container::Ivo, 0xF735_1608);
}

/// The known chunk types of the `CrCh` container.
pub mod crch {
    use crate::{ChunkType, Container};

    /// The `Mesh` chunk type.
    pub const MESH: ChunkType = ChunkType::new(Container::CrCh, 0x1000);
    /// The `Helper` chunk type.
    pub const HELPER: ChunkType = ChunkType::new(Container::CrCh, 0x1001);
    /// The `VertAnim` chunk type.
    pub const VERT_ANIM: ChunkType = ChunkType::new(Container::CrCh, 0x1002);
    /// The `BoneAnim` chunk type.
    pub const BONE_ANIM: ChunkType = ChunkType::new(Container::CrCh, 0x1003);
    /// The `GeomNameList` chunk type.
    pub const GEOM_NAME_LIST: ChunkType = ChunkType::new(Container::CrCh, 0x1004);
    /// The `BoneNameList` chunk type.
    pub const BONE_NAME_LIST: ChunkType = ChunkType::new(Container::CrCh, 0x1005);
    /// The `MtlList` chunk type.
    pub const MTL_LIST: ChunkType = ChunkType::new(Container::CrCh, 0x1006);
    /// The `Mrm` chunk type.
    pub const MRM: ChunkType = ChunkType::new(Container::CrCh, 0x1007);
    /// The `SceneProps` chunk type.
    pub const SCENE_PROPS: ChunkType = ChunkType::new(Container::CrCh, 0x1008);
    /// The `Light` chunk type.
    pub const LIGHT: ChunkType = ChunkType::new(Container::CrCh, 0x1009);
    /// The `PatchMesh` chunk type.
    pub const PATCH_MESH: ChunkType = ChunkType::new(Container::CrCh, 0x100A);
    /// The `Node` chunk type.
    pub const NODE: ChunkType = ChunkType::new(Container::CrCh, 0x100B);
    /// The `Mtl` chunk type.
    pub const MTL: ChunkType = ChunkType::new(Container::CrCh, 0x100C);
    /// The `Controller` chunk type.
    pub const CONTROLLER: ChunkType = ChunkType::new(Container::CrCh, 0x100D);
    /// The `Timing` chunk type.
    pub const TIMING: ChunkType = ChunkType::new(Container::CrCh, 0x100E);
    /// The `BoneMesh` chunk type.
    pub const BONE_MESH: ChunkType = ChunkType::new(Container::CrCh, 0x100F);
    /// The `BoneLightBinding` chunk type.
    pub const BONE_LIGHT_BINDING: ChunkType = ChunkType::new(Container::CrCh, 0x1010);
    /// The `MeshMorphTarget` chunk type.
    pub const MESH_MORPH_TARGET: ChunkType = ChunkType::new(Container::CrCh, 0x1011);
    /// The `BoneInitialPos` chunk type.
    pub const BONE_INITIAL_POS: ChunkType = ChunkType::new(Container::CrCh, 0x1012);
    /// The `SourceInfo` chunk type.
    pub const SOURCE_INFO: ChunkType = ChunkType::new(Container::CrCh, 0x1013);
    /// The `MtlName` chunk type.
    pub const MTL_NAME: ChunkType = ChunkType::new(Container::CrCh, 0x1014);
    /// The `ExportFlags` chunk type.
    pub const EXPORT_FLAGS: ChunkType = ChunkType::new(Container::CrCh, 0x1015);
    /// The `DataStream` chunk type.
    pub const DATA_STREAM: ChunkType = ChunkType::new(Container::CrCh, 0x1016);
    /// The `MeshSubsets` chunk type.
    pub const MESH_SUBSETS: ChunkType = ChunkType::new(Container::CrCh, 0x1017);
    /// The `MeshPhysicsData` chunk type.
    pub const MESH_PHYSICS_DATA: ChunkType = ChunkType::new(Container::CrCh, 0x1018);
    /// The `CompiledBones` chunk type.
    pub const COMPILED_BONES: ChunkType = ChunkType::new(Container::CrCh, 0x2000);
    /// The `CompiledPhysicalBones` chunk type.
    pub const COMPILED_PHYSICAL_BONES: ChunkType = ChunkType::new(Container::CrCh, 0x2001);
    /// The `CompiledMorphTargets` chunk type.
    pub const COMPILED_MORPH_TARGETS: ChunkType = ChunkType::new(Container::CrCh, 0x2002);
    /// The `CompiledPhysicalProxies` chunk type.
    pub const COMPILED_PHYSICAL_PROXIES: ChunkType = ChunkType::new(Container::CrCh, 0x2003);
    /// The `CompiledIntFaces` chunk type.
    pub const COMPILED_INT_FACES: ChunkType = ChunkType::new(Container::CrCh, 0x2004);
    /// The `CompiledIntSkinVertices` chunk type.
    pub const COMPILED_INT_SKIN_VERTICES: ChunkType = ChunkType::new(Container::CrCh, 0x2005);
    /// The `CompiledExt2IntMap` chunk type.
    pub const COMPILED_EXT2_INT_MAP: ChunkType = ChunkType::new(Container::CrCh, 0x2006);
    /// The `BreakablePhysics` chunk type.
    pub const BREAKABLE_PHYSICS: ChunkType = ChunkType::new(Container::CrCh, 0x3000);
    /// The `FaceMap` chunk type.
    pub const FACE_MAP: ChunkType = ChunkType::new(Container::CrCh, 0x3001);
    /// The `MotionParameters` chunk type.
    pub const MOTION_PARAMETERS: ChunkType = ChunkType::new(Container::CrCh, 0x3002);
    /// The `FootPlantInfo` chunk type.
    pub const FOOT_PLANT_INFO: ChunkType = ChunkType::new(Container::CrCh, 0x3003);
    /// The `BonesBoxes` chunk type.
    pub const BONES_BOXES: ChunkType = ChunkType::new(Container::CrCh, 0x3004);
    /// The `FoliageInfo` chunk type.
    pub const FOLIAGE_INFO: ChunkType = ChunkType::new(Container::CrCh, 0x3005);
    /// The `Timestamp` chunk type.
    pub const TIMESTAMP: ChunkType = ChunkType::new(Container::CrCh, 0x3006);
    /// The `GlobalAnimationHeaderCaf` chunk type.
    pub const GLOBAL_ANIMATION_HEADER_CAF: ChunkType = ChunkType::new(Container::CrCh, 0x3007);
    /// The `GlobalAnimationHeaderAim` chunk type.
    pub const GLOBAL_ANIMATION_HEADER_AIM: ChunkType = ChunkType::new(Container::CrCh, 0x3008);
    /// The `BspTreeData` chunk type.
    pub const BSP_TREE_DATA: ChunkType = ChunkType::new(Container::CrCh, 0x3009);
}
