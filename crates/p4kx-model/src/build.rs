use p4kx_chunks::{ChunkFile, ChunkType, ivo};

use crate::material::material_library;
use crate::mesh::split_by_node;
use crate::model::MESH_TYPES;
use crate::nmc::read_nodes;
use crate::skin::read_skin;
use crate::{
    ChunkUsage, CompanionPath, CompanionResolver, CorruptChunk, Lod, LodSelection, Mesh, Model,
    ModelChunk, ModelError, ModelFile, Node, Transform,
};

const MATERIAL_TYPES: [ChunkType; 2] = [ivo::MTL_NAME_320, ivo::MTL_NAME];

const MAX_LOD: u32 = 15;

type Companion = (String, ChunkFile<Vec<u8>>);

pub(crate) fn build<B: AsRef<[u8]>>(
    name: &str,
    file: &ChunkFile<B>,
    companions: &mut dyn CompanionResolver,
    selection: LodSelection,
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

    let Geometry {
        lods,
        base,
        lod_files,
    } = read_geometry(name, file, companions, selection, node_count, &mut read)?;

    let mut chunks = usage(file, ModelFile::Model, &read);
    if let Some((_, companion)) = &base {
        chunks.extend(usage(companion, ModelFile::Geometry, &read));
    }
    for (level, (_, lod_file)) in &lod_files {
        chunks.extend(usage(lod_file, ModelFile::LodGeometry(*level), &read));
    }
    Ok(Model {
        name: name.to_owned(),
        geometry: base.map(|(companion_name, _)| companion_name),
        nodes: nodes.unwrap_or_else(|| vec![root_node(name)]),
        lods,
        material_library: material.flatten(),
        bones: Vec::new(),
        chunks,
    })
}

/// The meshes of a model and the companions they were read from.
struct Geometry {
    lods: Vec<Lod>,
    base: Option<Companion>,
    lod_files: Vec<(u32, Companion)>,
}

fn read_geometry<B: AsRef<[u8]>>(
    name: &str,
    file: &ChunkFile<B>,
    companions: &mut dyn CompanionResolver,
    selection: LodSelection,
    node_count: Option<usize>,
    read: &mut Vec<(ModelFile, usize)>,
) -> Result<Geometry, ModelError> {
    let own_mesh = first_of(file, &MESH_TYPES);
    let base = if own_mesh.is_some() || !selection.wants(0) {
        None
    } else {
        geometry_companion(name, file, companions)?
    };
    let mut lods = Vec::new();
    let mut lod_files: Vec<(u32, Companion)> = Vec::new();
    if selection.wants(0) {
        lods.push(read_base_lod(
            name,
            file,
            base.as_ref(),
            own_mesh,
            node_count,
            read,
        )?);
    }
    if own_mesh.is_none() && announces_mesh(file) {
        read_lod_companions(
            name,
            companions,
            selection,
            node_count,
            &mut lods,
            &mut lod_files,
            read,
        )?;
    }
    if let LodSelection::Only(level) = selection
        && lods.is_empty()
    {
        return Err(ModelError::NoSuchLod {
            file: name.to_owned(),
            lod: level,
        });
    }

    Ok(Geometry {
        lods,
        base,
        lod_files,
    })
}

fn read_base_lod<B: AsRef<[u8]>>(
    name: &str,
    file: &ChunkFile<B>,
    base: Option<&Companion>,
    own_mesh: Option<usize>,
    node_count: Option<usize>,
    read: &mut Vec<(ModelFile, usize)>,
) -> Result<Lod, ModelError> {
    let meshes = match (base, own_mesh) {
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
    Ok(Lod { level: 0, meshes })
}

/// Returns the file name of the geometry companion of level of detail `level` of the model
/// `name`: `<stem>_lod<level>.<extension>m`.
fn lod_companion_name(name: &str, level: u32) -> String {
    match name.rsplit_once('.') {
        Some((stem, extension)) => format!("{stem}_lod{level}.{extension}m"),
        None => format!("{name}_lod{level}m"),
    }
}

fn read_lod_companions(
    name: &str,
    companions: &mut dyn CompanionResolver,
    selection: LodSelection,
    nodes: Option<usize>,
    lods: &mut Vec<Lod>,
    lod_files: &mut Vec<(u32, Companion)>,
    read: &mut Vec<(ModelFile, usize)>,
) -> Result<(), ModelError> {
    for level in (1..=MAX_LOD).filter(|&level| selection.wants(level)) {
        let companion_name = lod_companion_name(name, level);
        let Some(companion) = fetch(companions, &companion_name)? else {
            continue;
        };
        let index = first_of(&companion, &MESH_TYPES).ok_or_else(|| ModelError::NoMesh {
            file: companion_name.clone(),
        })?;
        read.push((ModelFile::LodGeometry(level), index));
        let meshes = read_meshes(&companion_name, index, companion.chunk_data(index), nodes)?;
        lods.push(Lod { level, meshes });
        lod_files.push((level, (companion_name, companion)));
    }
    Ok(())
}

fn announces_mesh<B: AsRef<[u8]>>(file: &ChunkFile<B>) -> bool {
    file.chunks_of(ivo::MESH_INFO).next().is_some()
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
