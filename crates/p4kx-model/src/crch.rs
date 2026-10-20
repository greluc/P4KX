use p4kx_chunks::{ByteOrder, ChunkFile, ChunkType, crch};

use crate::build::{corrupt, root_node, usage};
use crate::crch_mesh::{physics_chunk_id, read_physics_mesh};
use crate::crch_node::{CrchNode, read_nodes};
use crate::material::material_library;
use crate::mtl_load::load;
use crate::{
    CompanionResolver, CorruptChunk, Lod, LodSelection, Mesh, Model, ModelError, ModelFile,
};

const MESH_VERSION: u32 = 0x801;
const PHYSICS_DATA_VERSION: u32 = 0x800;
const MTL_NAME_VERSION: u32 = 0x802;

/// Builds the model of a `CrCh` file: its nodes, the meshes of the nodes that refer to a `Mesh`
/// chunk and the material library its `MtlName` chunk names. A `CrCh` model has one level of
/// detail and no geometry companion.
pub(crate) fn build<B: AsRef<[u8]>>(
    name: &str,
    file: &ChunkFile<B>,
    companions: &mut dyn CompanionResolver,
    selection: LodSelection,
) -> Result<Model, ModelError> {
    let mut read = Vec::new();
    let crch_nodes = read_nodes(name, file, &mut read)?;
    let mut lods = Vec::new();
    if selection.wants(0) {
        let mut meshes = Vec::new();
        for (position, node) in crch_nodes.iter().enumerate() {
            if let Some(mesh) = read_node_mesh(name, file, position, node, &mut read)? {
                meshes.push(mesh);
            }
        }
        lods.push(Lod {
            level: 0,
            meshes,
            bones: None,
        });
    }
    if let LodSelection::Only(level) = selection
        && lods.is_empty()
    {
        return Err(ModelError::NoSuchLod {
            file: name.to_owned(),
            lod: level,
        });
    }

    let library = first_of_version(file, crch::MTL_NAME, MTL_NAME_VERSION).and_then(|index| {
        read.push((ModelFile::Model, index));
        material_library(file.chunk_data(index).unwrap_or_default())
    });
    let materials = load(library.as_deref(), companions);
    let mut nodes: Vec<_> = crch_nodes.into_iter().map(|crch| crch.node).collect();
    if nodes.is_empty() {
        nodes.push(root_node(name));
    }
    Ok(Model {
        name: name.to_owned(),
        geometry: None,
        nodes,
        lods,
        material_library: library,
        materials,
        bones: Vec::new(),
        chunks: usage(file, ModelFile::Model, &read),
    })
}

fn first_of_version<B: AsRef<[u8]>>(
    file: &ChunkFile<B>,
    chunk_type: ChunkType,
    version: u32,
) -> Option<usize> {
    file.chunks_of(chunk_type)
        .find(|(_, chunk)| chunk.version() == version)
        .map(|(index, _)| index)
}

fn read_node_mesh<B: AsRef<[u8]>>(
    name: &str,
    file: &ChunkFile<B>,
    position: usize,
    node: &CrchNode,
    read: &mut Vec<(ModelFile, usize)>,
) -> Result<Option<Mesh>, ModelError> {
    let Some((mesh_index, mesh_chunk)) = file.chunk_by_id(node.object) else {
        return Ok(None);
    };
    if mesh_chunk.chunk_type() != crch::MESH || mesh_chunk.version() != MESH_VERSION {
        return Ok(None);
    }
    let fail = corrupt(name, mesh_index, "Mesh");
    if mesh_chunk.byte_order() != ByteOrder::Little {
        return Err(fail(CorruptChunk::BigEndian));
    }
    read.push((ModelFile::Model, mesh_index));
    let Some(physics_id) =
        physics_chunk_id(file.chunk_data(mesh_index).unwrap_or_default()).map_err(&fail)?
    else {
        return Ok(None);
    };

    let physics_index = file
        .chunk_by_id(physics_id)
        .filter(|(_, chunk)| {
            chunk.chunk_type() == crch::MESH_PHYSICS_DATA && chunk.version() == PHYSICS_DATA_VERSION
        })
        .map(|(index, _)| index)
        .ok_or_else(|| fail(CorruptChunk::ReferencedChunk(physics_id)))?;
    let fail = corrupt(name, physics_index, "MeshPhysicsData");
    if file
        .chunks()
        .get(physics_index)
        .is_none_or(|chunk| chunk.byte_order() != ByteOrder::Little)
    {
        return Err(fail(CorruptChunk::BigEndian));
    }
    read.push((ModelFile::Model, physics_index));
    read_physics_mesh(position, file.chunk_data(physics_index).unwrap_or_default()).map_err(fail)
}
