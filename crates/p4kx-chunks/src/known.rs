// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::types::{crch, ivo};
use crate::{ChunkType, Container};

/// One entry of the table of known chunk types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KnownChunkType {
    /// The numeric type ID: a hash in `#ivo`, a 16-bit ID in `CrCh`.
    pub id: u32,
    /// The name of the type.
    pub name: &'static str,
}

impl KnownChunkType {
    /// Returns the known chunk types of `container`, in ascending ID order.
    #[must_use]
    pub const fn table(container: Container) -> &'static [Self] {
        match container {
            Container::Ivo => IVO,
            Container::CrCh => CRCH,
        }
    }
}

pub(crate) fn name(container: Container, id: u32) -> Option<&'static str> {
    let table = KnownChunkType::table(container);
    table
        .binary_search_by_key(&id, |known| known.id)
        .ok()
        .and_then(|index| table.get(index))
        .map(|known| known.name)
}

const fn known(chunk_type: ChunkType, name: &'static str) -> KnownChunkType {
    KnownChunkType {
        id: chunk_type.id(),
        name,
    }
}

const IVO: &[KnownChunkType] = &[
    known(ivo::RIG_LOGIC, "RigLogic"),
    known(ivo::DBA_DATA, "DbaData"),
    known(ivo::SKELETON, "Skeleton"),
    known(ivo::VFX_HELPERS, "VfxHelpers"),
    known(ivo::POSITION_BONEMAP, "PositionBonemap"),
    known(ivo::SIM_TOPOLOGY, "SimTopology"),
    known(ivo::ANIM_INFO, "AnimInfo"),
    known(ivo::BSHAPES_GPU, "BShapesGpu"),
    known(ivo::BSHAPES_GPU_EARLY, "BShapesGpu"),
    known(ivo::STAT_OBJ_PHYSICS, "StatObjPhysics"),
    known(ivo::NODE_MESH_COMBO, "NodeMeshCombo"),
    known(ivo::RIG_INFO, "RigInfo"),
    known(ivo::MTL_NAME_320, "MtlName320"),
    known(ivo::MTL_NAME_VARIANT, "MtlNameVariant"),
    known(ivo::MTL_NAME, "MtlName"),
    known(ivo::BSHAPES_EARLY, "BShapes"),
    known(ivo::PHYSICAL_HIERARCHY, "PhysicalHierarchy"),
    known(ivo::COMPILED_PHYSICAL_BONES_320, "CompiledPhysicalBones320"),
    known(ivo::COMPILED_PHYSICAL_BONES, "CompiledPhysicalBones"),
    known(ivo::MESH_INFO, "MeshInfo"),
    known(ivo::MESH, "Mesh"),
    known(ivo::LOD_DISTANCES, "LodDistances"),
    known(ivo::CAF_DATA, "CafData"),
    known(ivo::VIS_AREAS, "VisAreas"),
    known(ivo::SKIN_2, "Skin2"),
    known(ivo::SKIN, "Skin"),
    known(ivo::EXPORT_FLAGS, "ExportFlags"),
    known(ivo::VISIBILITY_GROUPS, "VisibilityGroups"),
    known(ivo::COMPILED_BONES_2, "CompiledBones2"),
    known(ivo::COMPILED_BONES, "CompiledBones"),
    known(ivo::TERMINATOR, "Terminator"),
    known(ivo::BSHAPES, "BShapes"),
    known(ivo::PROTOS_INFO, "ProtosInfo"),
    known(ivo::DBA_METADATA, "DbaMetadata"),
    known(ivo::SIM_DEFORMATION, "SimDeformation"),
];

const CRCH: &[KnownChunkType] = &[
    known(crch::MESH, "Mesh"),
    known(crch::HELPER, "Helper"),
    known(crch::VERT_ANIM, "VertAnim"),
    known(crch::BONE_ANIM, "BoneAnim"),
    known(crch::GEOM_NAME_LIST, "GeomNameList"),
    known(crch::BONE_NAME_LIST, "BoneNameList"),
    known(crch::MTL_LIST, "MtlList"),
    known(crch::MRM, "Mrm"),
    known(crch::SCENE_PROPS, "SceneProps"),
    known(crch::LIGHT, "Light"),
    known(crch::PATCH_MESH, "PatchMesh"),
    known(crch::NODE, "Node"),
    known(crch::MTL, "Mtl"),
    known(crch::CONTROLLER, "Controller"),
    known(crch::TIMING, "Timing"),
    known(crch::BONE_MESH, "BoneMesh"),
    known(crch::BONE_LIGHT_BINDING, "BoneLightBinding"),
    known(crch::MESH_MORPH_TARGET, "MeshMorphTarget"),
    known(crch::BONE_INITIAL_POS, "BoneInitialPos"),
    known(crch::SOURCE_INFO, "SourceInfo"),
    known(crch::MTL_NAME, "MtlName"),
    known(crch::EXPORT_FLAGS, "ExportFlags"),
    known(crch::DATA_STREAM, "DataStream"),
    known(crch::MESH_SUBSETS, "MeshSubsets"),
    known(crch::MESH_PHYSICS_DATA, "MeshPhysicsData"),
    known(crch::COMPILED_BONES, "CompiledBones"),
    known(crch::COMPILED_PHYSICAL_BONES, "CompiledPhysicalBones"),
    known(crch::COMPILED_MORPH_TARGETS, "CompiledMorphTargets"),
    known(crch::COMPILED_PHYSICAL_PROXIES, "CompiledPhysicalProxies"),
    known(crch::COMPILED_INT_FACES, "CompiledIntFaces"),
    known(crch::COMPILED_INT_SKIN_VERTICES, "CompiledIntSkinVertices"),
    known(crch::COMPILED_EXT2_INT_MAP, "CompiledExt2IntMap"),
    known(crch::BREAKABLE_PHYSICS, "BreakablePhysics"),
    known(crch::FACE_MAP, "FaceMap"),
    known(crch::MOTION_PARAMETERS, "MotionParameters"),
    known(crch::FOOT_PLANT_INFO, "FootPlantInfo"),
    known(crch::BONES_BOXES, "BonesBoxes"),
    known(crch::FOLIAGE_INFO, "FoliageInfo"),
    known(crch::TIMESTAMP, "Timestamp"),
    known(
        crch::GLOBAL_ANIMATION_HEADER_CAF,
        "GlobalAnimationHeaderCaf",
    ),
    known(
        crch::GLOBAL_ANIMATION_HEADER_AIM,
        "GlobalAnimationHeaderAim",
    ),
    known(crch::BSP_TREE_DATA, "BspTreeData"),
];
