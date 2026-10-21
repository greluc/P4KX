use p4kx_chunks::{ChunkFile, ChunkType, ivo};

use crate::bone_map::check_joints;
use crate::material::material_library;
use crate::mesh::split_by_node;
use crate::model::MESH_TYPES;
use crate::mtl_load::load;
use crate::nmc::read_nodes;
use crate::skeleton::{read_bones, read_compact_bones};
use crate::skin::read_skin;
use crate::{
    Bone, ChunkUsage, CompanionPath, CompanionResolver, CorruptChunk, Lod, LodSelection, Mesh,
    Model, ModelChunk, ModelError, ModelFile, Node, Transform,
};

const MATERIAL_TYPES: [ChunkType; 2] = [ivo::MTL_NAME_320, ivo::MTL_NAME];

const MAX_LOD: u32 = 15;

type Companion = (String, ChunkFile<Vec<u8>>);

/// What the meshes of a model are checked against: its node and bone counts.
#[derive(Debug, Clone, Copy)]
struct Rig<'a> {
    nodes: Option<usize>,
    bones: &'a [Bone],
}

fn read_skeleton<B: AsRef<[u8]>>(
    name: &str,
    file: &ChunkFile<B>,
    read: &mut Vec<(ModelFile, usize)>,
) -> Result<Vec<Bone>, ModelError> {
    let Some((index, chunk)) = [ivo::COMPILED_BONES_2, ivo::COMPILED_BONES]
        .into_iter()
        .find_map(|chunk_type| file.chunks_of(chunk_type).next())
    else {
        return Ok(Vec::new());
    };
    read.push((ModelFile::Model, index));
    let data = file.chunk_data(index).unwrap_or_default();
    let (chunk_name, bones) = if chunk.chunk_type() == ivo::COMPILED_BONES_2 {
        ("CompiledBones2", read_bones(data))
    } else {
        ("CompiledBones", read_compact_bones(data))
    };
    bones.map_err(corrupt(name, index, chunk_name))
}

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
    let bones = read_skeleton(name, file, &mut read)?;
    let rig = Rig {
        nodes: nodes.as_ref().map(Vec::len),
        bones: &bones,
    };
    let material = first_of(file, &MATERIAL_TYPES).map(|index| {
        read.push((ModelFile::Model, index));
        material_library(file.chunk_data(index).unwrap_or_default())
    });

    let Geometry {
        lods,
        base_name,
        chunks: companion_chunks,
    } = read_geometry(name, file, companions, selection, rig, &mut read)?;

    let mut chunks = usage(file, ModelFile::Model, &read);
    chunks.extend(companion_chunks);
    let material_library = material.flatten();
    let materials = load(material_library.as_deref(), companions);
    Ok(Model {
        name: name.to_owned(),
        geometry: base_name,
        nodes: nodes.unwrap_or_else(|| vec![root_node(name)]),
        lods,
        material_library,
        materials,
        bones,
        chunks,
    })
}

/// The meshes of a model, the name of its geometry companion and the chunk usage of every
/// companion, computed as each was read so that no companion's bytes outlive its reading.
struct Geometry {
    lods: Vec<Lod>,
    base_name: Option<String>,
    chunks: Vec<ModelChunk>,
}

fn read_geometry<B: AsRef<[u8]>>(
    name: &str,
    file: &ChunkFile<B>,
    companions: &mut dyn CompanionResolver,
    selection: LodSelection,
    rig: Rig<'_>,
    read: &mut Vec<(ModelFile, usize)>,
) -> Result<Geometry, ModelError> {
    let own_mesh = first_of(file, &MESH_TYPES);
    let base = if own_mesh.is_some() || !selection.wants(0) {
        None
    } else {
        geometry_companion(name, file, companions)?
    };
    let mut lods = Vec::new();
    let mut chunks = Vec::new();
    let mut geometry = None;
    if selection.wants(0) {
        lods.push(read_base_lod(
            name,
            file,
            base.as_ref(),
            own_mesh,
            rig,
            read,
        )?);
    }
    if let Some((companion_name, companion)) = base {
        chunks.extend(usage(&companion, ModelFile::Geometry, read));
        geometry = Some(companion_name);
    }
    if own_mesh.is_none() && announces_mesh(file) {
        read_lod_companions(
            name,
            companions,
            selection,
            rig,
            &mut lods,
            &mut chunks,
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
        base_name: geometry,
        chunks,
    })
}

fn read_base_lod<B: AsRef<[u8]>>(
    name: &str,
    file: &ChunkFile<B>,
    base: Option<&Companion>,
    own_mesh: Option<usize>,
    rig: Rig<'_>,
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
                rig.nodes,
                Some(rig.bones.len()),
            )?
        }
        (None, Some(index)) => {
            read.push((ModelFile::Model, index));
            let data = file.chunk_data(index);
            read_meshes(name, index, data, rig.nodes, Some(rig.bones.len()))?
        }
        (None, None) => Vec::new(),
    };
    Ok(Lod {
        level: 0,
        meshes,
        bones: None,
    })
}

/// Returns the names of the files next to the model file `name` that reading it can fetch: the
/// geometry companion `<name>m`, and for every level of detail from 1 on its model file and its
/// geometry companion.
#[must_use]
pub fn companion_file_names(name: &str) -> Vec<String> {
    let mut names = vec![format!("{name}m")];
    for level in 1..=MAX_LOD {
        let file = lod_file_name(name, level);
        names.push(format!("{file}m"));
        names.push(file);
    }
    names
}

/// Returns the file name of the model file of level of detail `level` of the model `name`:
/// `<stem>_lod<level>.<extension>`.
fn lod_file_name(name: &str, level: u32) -> String {
    match name.rsplit_once('.') {
        Some((stem, extension)) => format!("{stem}_lod{level}.{extension}"),
        None => format!("{name}_lod{level}"),
    }
}

/// Reads the skeleton of the model file of `level`, which a skinned level's bone indices refer
/// to; `None` when the file is missing, holds no skeleton or the one `base` already is.
fn level_skeleton(
    name: &str,
    level: u32,
    base: &[Bone],
    companions: &mut dyn CompanionResolver,
) -> Result<Option<Vec<Bone>>, ModelError> {
    let file_name = lod_file_name(name, level);
    let Some(file) = fetch(companions, &file_name)? else {
        return Ok(None);
    };
    let bones = read_skeleton(&file_name, &file, &mut Vec::new())?;
    Ok((!bones.is_empty() && bones != base).then_some(bones))
}

fn read_lod_companions(
    name: &str,
    companions: &mut dyn CompanionResolver,
    selection: LodSelection,
    rig: Rig<'_>,
    lods: &mut Vec<Lod>,
    chunks: &mut Vec<ModelChunk>,
    read: &mut Vec<(ModelFile, usize)>,
) -> Result<(), ModelError> {
    for level in (1..=MAX_LOD).filter(|&level| selection.wants(level)) {
        let companion_name = format!("{}m", lod_file_name(name, level));
        let Some(companion) = fetch(companions, &companion_name)? else {
            continue;
        };
        let index = first_of(&companion, &MESH_TYPES).ok_or_else(|| ModelError::NoMesh {
            file: companion_name.clone(),
        })?;
        read.push((ModelFile::LodGeometry(level), index));
        let meshes = read_meshes(
            &companion_name,
            index,
            companion.chunk_data(index),
            rig.nodes,
            None,
        )?;
        let bones = if meshes.iter().any(|mesh| mesh.influences.is_some()) {
            level_skeleton(name, level, rig.bones, companions)?
        } else {
            None
        };
        let count = bones.as_ref().map_or(rig.bones.len(), Vec::len);
        check_joints(&meshes, count).map_err(corrupt(&companion_name, index, "Skin"))?;
        lods.push(Lod {
            level,
            meshes,
            bones,
        });
        chunks.extend(usage(&companion, ModelFile::LodGeometry(level), read));
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
    bones: Option<usize>,
) -> Result<Vec<Mesh>, ModelError> {
    let corrupt = corrupt(file, index, "Skin");
    let skin = read_skin(data.unwrap_or_default()).map_err(&corrupt)?;
    let meshes = split_by_node(&skin, nodes).map_err(&corrupt)?;
    if let Some(bones) = bones {
        check_joints(&meshes, bones).map_err(&corrupt)?;
    }
    Ok(meshes)
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

pub(crate) fn usage<B: AsRef<[u8]>>(
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

pub(crate) fn root_node(name: &str) -> Node {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    Node {
        name: stem.to_owned(),
        parent: None,
        transform: Transform::IDENTITY,
    }
}

pub(crate) fn corrupt(
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
