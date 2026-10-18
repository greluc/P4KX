use crate::Container;

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

const fn known(id: u32, name: &'static str) -> KnownChunkType {
    KnownChunkType { id, name }
}

const IVO: &[KnownChunkType] = &[
    known(0x194F_BC50, "DbaData"),
    known(0x2B7E_CF9F, "BoundingData"),
    known(0x4733_C6ED, "AnimInfo"),
    known(0x57A3_BEFD, "BShapesGpu"),
    known(0x58DE_1772, "LodMeshData"),
    known(0x7069_7FDA, "NodeMeshCombo"),
    known(0x8335_3333, "MtlName320"),
    known(0x8335_3533, "MtlNameVariant"),
    known(0x8335_674E, "MtlName"),
    known(0x875C_CB28, "BShapes"),
    known(0x90C6_6666, "CompiledPhysicalBones320"),
    known(0x90C6_87DC, "CompiledPhysicalBones"),
    known(0x9291_4444, "MeshInfo"),
    known(0x9293_B9D8, "Mesh"),
    known(0x9351_756F, "LodDistances"),
    known(0xA949_6CB5, "CafData"),
    known(0xB875_7777, "Skin2"),
    known(0xB875_B2D9, "Skin"),
    known(0xBE5E_493E, "AssetMetadata"),
    known(0xC201_1111, "CompiledBones2"),
    known(0xC201_973C, "CompiledBones"),
    known(0xE018_1074, "Terminator"),
    known(0xF735_1608, "DbaMetadata"),
];

const CRCH: &[KnownChunkType] = &[
    known(0x1000, "Mesh"),
    known(0x1001, "Helper"),
    known(0x1002, "VertAnim"),
    known(0x1003, "BoneAnim"),
    known(0x1004, "GeomNameList"),
    known(0x1005, "BoneNameList"),
    known(0x1006, "MtlList"),
    known(0x1007, "Mrm"),
    known(0x1008, "SceneProps"),
    known(0x1009, "Light"),
    known(0x100A, "PatchMesh"),
    known(0x100B, "Node"),
    known(0x100C, "Mtl"),
    known(0x100D, "Controller"),
    known(0x100E, "Timing"),
    known(0x100F, "BoneMesh"),
    known(0x1010, "BoneLightBinding"),
    known(0x1011, "MeshMorphTarget"),
    known(0x1012, "BoneInitialPos"),
    known(0x1013, "SourceInfo"),
    known(0x1014, "MtlName"),
    known(0x1015, "ExportFlags"),
    known(0x1016, "DataStream"),
    known(0x1017, "MeshSubsets"),
    known(0x1018, "MeshPhysicsData"),
    known(0x2000, "CompiledBones"),
    known(0x2001, "CompiledPhysicalBones"),
    known(0x2002, "CompiledMorphTargets"),
    known(0x2003, "CompiledPhysicalProxies"),
    known(0x2004, "CompiledIntFaces"),
    known(0x2005, "CompiledIntSkinVertices"),
    known(0x2006, "CompiledExt2IntMap"),
    known(0x3000, "BreakablePhysics"),
    known(0x3001, "FaceMap"),
    known(0x3002, "MotionParameters"),
    known(0x3003, "FootPlantInfo"),
    known(0x3004, "BonesBoxes"),
    known(0x3005, "FoliageInfo"),
    known(0x3006, "Timestamp"),
    known(0x3007, "GlobalAnimationHeaderCaf"),
    known(0x3008, "GlobalAnimationHeaderAim"),
    known(0x3009, "BspTreeData"),
];
