use p4kx_chunks::{ChunkFile, ChunkType, ivo};

use crate::material::material_library;
use crate::mesh::split_by_node;
use crate::model::MESH_TYPES;
use crate::nmc::read_nodes;
use crate::skin::read_skin;
use crate::{
    ChunkUsage, CompanionPath, CompanionResolver, CorruptChunk, Lod, Mesh, Model, ModelChunk,
    ModelError, ModelFile, Node, Transform,
};

const MATERIAL_TYPES: [ChunkType; 2] = [ivo::MTL_NAME_320, ivo::MTL_NAME];

type Companion = (String, ChunkFile<Vec<u8>>);

pub(crate) fn build<B: AsRef<[u8]>>(
    name: &str,
    file: &ChunkFile<B>,
    companions: &mut dyn CompanionResolver,
) -> Result<Model, ModelError> {
    let mut read = Vec::new();
    let nodes = match file.chunks_of(ivo::NODE_MESH_COMBO).next() {
        Some((index, _)) => {
            read.push((ModelFile::Model, index));
            let data = file.chunk_data(index).unwrap_or_default();
            Some(read_nodes(data).map_err(corrupt(name, index, "NodeMeshCombo"))?)
        }
        None => None,
    }
    .filter(|nodes| !nodes.is_empty());
    let node_count = nodes.as_ref().map(Vec::len);
    let material = first_of(file, &MATERIAL_TYPES).map(|index| {
        read.push((ModelFile::Model, index));
        material_library(file.chunk_data(index).unwrap_or_default())
    });

    let own_mesh = first_of(file, &MESH_TYPES);
    let companion = if own_mesh.is_some() {
        None
    } else {
        geometry_companion(name, file, companions)?
    };
    let meshes = match (&companion, own_mesh) {
        (Some((companion_name, companion)), _) => {
            let index = first_of(companion, &MESH_TYPES).ok_or_else(|| ModelError::NoMesh {
                file: companion_name.clone(),
            })?;
            read.push((ModelFile::Geometry, index));
            read_meshes(
                companion_name,
                index,
                companion.chunk_data(index),
                node_count,
            )?
        }
        (None, Some(index)) => {
            read.push((ModelFile::Model, index));
            read_meshes(name, index, file.chunk_data(index), node_count)?
        }
        (None, None) => Vec::new(),
    };

    let mut chunks = usage(file, ModelFile::Model, &read);
    if let Some((_, companion)) = &companion {
        chunks.extend(usage(companion, ModelFile::Geometry, &read));
    }
    Ok(Model {
        name: name.to_owned(),
        geometry: companion.map(|(companion_name, _)| companion_name),
        nodes: nodes.unwrap_or_else(|| vec![root_node(name)]),
        lods: vec![Lod { level: 0, meshes }],
        material_library: material.flatten(),
        bones: Vec::new(),
        chunks,
    })
}

fn geometry_companion<B: AsRef<[u8]>>(
    name: &str,
    file: &ChunkFile<B>,
    companions: &mut dyn CompanionResolver,
) -> Result<Option<Companion>, ModelError> {
    let companion_name = format!("{name}m");
    let announced = file.chunks_of(ivo::MESH_INFO).next().is_some();
    match fetch(companions, &companion_name)? {
        Some(companion) => Ok(Some((companion_name, companion))),
        None if announced => Err(ModelError::MissingGeometry {
            file: companion_name,
        }),
        None => Ok(None),
    }
}

fn read_meshes(
    file: &str,
    index: usize,
    data: Option<&[u8]>,
    nodes: Option<usize>,
) -> Result<Vec<Mesh>, ModelError> {
    let corrupt = corrupt(file, index, "Skin");
    let skin = read_skin(data.unwrap_or_default()).map_err(&corrupt)?;
    split_by_node(&skin, nodes).map_err(&corrupt)
}

fn fetch(
    companions: &mut dyn CompanionResolver,
    name: &str,
) -> Result<Option<ChunkFile<Vec<u8>>>, ModelError> {
    let bytes = companions
        .resolve(CompanionPath::Sibling(name))
        .map_err(|source| ModelError::Companion {
            file: name.to_owned(),
            source,
        })?;
    bytes
        .map(ChunkFile::parse)
        .transpose()
        .map_err(|source| ModelError::CompanionChunks {
            file: name.to_owned(),
            source,
        })
}
fn first_of<B: AsRef<[u8]>>(file: &ChunkFile<B>, types: &[ChunkType]) -> Option<usize> {
    types
        .iter()
        .find_map(|&chunk_type| file.chunks_of(chunk_type).next())
        .map(|(index, _)| index)
}

fn usage<B: AsRef<[u8]>>(
    file: &ChunkFile<B>,
    which: ModelFile,
    read: &[(ModelFile, usize)],
) -> Vec<ModelChunk> {
    file.chunks()
        .iter()
        .enumerate()
        .map(|(index, chunk)| {
            let chunk_type = chunk.chunk_type();
            let usage = if read.contains(&(which, index)) {
                ChunkUsage::Read
            } else if chunk_type.is_known() {
                ChunkUsage::Skipped
            } else {
                ChunkUsage::Unknown
            };
            ModelChunk {
                file: which,
                index,
                chunk_type,
                usage,
            }
        })
        .collect()
}

fn root_node(name: &str) -> Node {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    Node {
        name: stem.to_owned(),
        parent: None,
        transform: Transform::IDENTITY,
    }
}

fn corrupt(
    file: &str,
    index: usize,
    chunk: &'static str,
) -> impl Fn(CorruptChunk) -> ModelError + use<> {
    let file = file.to_owned();
    move |source| ModelError::Corrupt {
        file: file.clone(),
        index,
        chunk,
        source,
    }
}
