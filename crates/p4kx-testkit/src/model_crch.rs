// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::bytes::put_u32;
use crate::model::{CUBE_MATERIAL_LIBRARY, CUBE_MESH_OFFSET, TestVertex};
use crate::{TestChunkFile, TestModel};

const MTL_NAME: u32 = 0x1014;
const HELPER: u32 = 0x1001;
const NODE: u32 = 0x100B;
const MESH: u32 = 0x1000;
const MESH_PHYSICS_DATA: u32 = 0x1018;
const UNKNOWN_TYPE: u32 = 0x300A;

const MTL_NAME_ID: u32 = 1;
const HELPER_ID: u32 = 2;
const PHYSICS_ID: u32 = 4;
const MESH_ID: u32 = 5;
const ROOT_ID: u32 = 3;
const NO_PARENT: u32 = u32::MAX;

const NAME_LEN: usize = 64;
const MTL_NAME_LEN: usize = 196;
const MESH_LEN: usize = 260;
const PHYSICS_HEADER_LEN: usize = 24;
const BLOB_PRIMITIVE_AT: usize = 0x44;
const BLOB_COUNTS_AT: usize = 0x48;
const BLOB_VERTICES_AT: usize = 0x5A;
const BLOB_TREE_LEN: usize = 64;

/// The translation of the root node of the [`TestCrchModel`] cube.
pub const CRCH_CUBE_ROOT_OFFSET: [f32; 3] = [5.0, 0.0, 0.0];

/// A synthetic `CrCh` model laid out like the model of the Archives.
///
/// A root node stands on a `Helper`; a geometry node stands on a `Mesh` chunk that refers to a
/// `MeshPhysicsData` chunk holding the triangles; a `MtlName` chunk and a chunk of unknown type
/// complete it.
///
/// [`Self::cube`] holds the 24 vertices and 12 triangles of [`TestModel::cube`]. The nodes carry
/// local matrices: the root is moved by [`CRCH_CUBE_ROOT_OFFSET`] and the geometry node, relative
/// to its parent, by [`CUBE_MESH_OFFSET`].
#[derive(Debug, Clone, PartialEq)]
pub struct TestCrchModel {
    vertices: Vec<TestVertex>,
    indices: Vec<u32>,
    material_library: String,
    declared_vertices: Option<u32>,
    streamed_vertices: u32,
    physics_primitive: u32,
    physics_version: u32,
    mesh_version: u32,
    extra_slots: Vec<(usize, u32)>,
    extra: Vec<(u32, u32, Vec<u8>)>,
}

impl TestCrchModel {
    /// Returns the cube.
    #[must_use]
    pub fn cube() -> Self {
        let cube = TestModel::cube();
        Self {
            vertices: cube.vertices,
            indices: cube.indices,
            material_library: CUBE_MATERIAL_LIBRARY.to_owned(),
            declared_vertices: None,
            streamed_vertices: 0,
            physics_primitive: 1,
            physics_version: 0x800,
            mesh_version: 0x801,
            extra_slots: Vec::new(),
            extra: Vec::new(),
        }
    }

    /// Writes the `MeshPhysicsData` chunk with the version `version`.
    #[must_use]
    pub const fn with_physics_version(mut self, version: u32) -> Self {
        self.physics_version = version;
        self
    }

    /// Writes the `Mesh` chunk with the version `version`.
    #[must_use]
    pub const fn with_mesh_version(mut self, version: u32) -> Self {
        self.mesh_version = version;
        self
    }

    /// Names the chunk ID `id` in physics slot `slot` (0 to 3) of the `Mesh` header.
    #[must_use]
    pub fn with_physics_slot(mut self, slot: usize, id: u32) -> Self {
        self.extra_slots.push((slot, id));
        self
    }

    /// Returns the vertices, in the geometry node's space.
    #[must_use]
    pub fn vertices(&self) -> &[TestVertex] {
        &self.vertices
    }

    /// Returns the triangle list.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Declares `count` vertices in the physics data header, whatever it holds.
    #[must_use]
    pub const fn declaring_vertices(mut self, count: u32) -> Self {
        self.declared_vertices = Some(count);
        self
    }

    /// Declares `count` vertices in data streams in the `Mesh` header.
    #[must_use]
    pub const fn streamed(mut self, count: u32) -> Self {
        self.streamed_vertices = count;
        self
    }

    /// Writes the physics data with the primitive type `primitive` at byte `0x44`.
    #[must_use]
    pub const fn with_physics_primitive(mut self, primitive: u32) -> Self {
        self.physics_primitive = primitive;
        self
    }

    /// Replaces the triangle index at `position`.
    #[must_use]
    pub fn with_index(mut self, position: usize, index: u32) -> Self {
        if let Some(slot) = self.indices.get_mut(position) {
            *slot = index;
        }
        self
    }

    /// Adds a chunk of any type at the end.
    #[must_use]
    pub fn with_chunk(mut self, type_id: u32, version: u32, data: impl Into<Vec<u8>>) -> Self {
        self.extra.push((type_id, version, data.into()));
        self
    }

    /// Returns the chunk file.
    #[must_use]
    pub fn file(&self) -> TestChunkFile {
        let [root_x, root_y, root_z] = CRCH_CUBE_ROOT_OFFSET;
        let [mesh_x, mesh_y, mesh_z] = CUBE_MESH_OFFSET;
        let mut file = TestChunkFile::crch()
            .chunk(MTL_NAME, 0x802, mtl_name_chunk(&self.material_library))
            .chunk(HELPER, 0x744, vec![0; 16])
            .chunk(
                NODE,
                0x824,
                node_chunk("cube", HELPER_ID, NO_PARENT, [root_x, root_y, root_z]),
            )
            .chunk(
                MESH_PHYSICS_DATA,
                self.physics_version,
                self.physics_chunk(),
            )
            .chunk(MESH, self.mesh_version, self.mesh_chunk())
            .chunk(
                NODE,
                0x824,
                node_chunk("cube_mesh", MESH_ID, ROOT_ID, [mesh_x, mesh_y, mesh_z]),
            )
            .chunk(UNKNOWN_TYPE, 1, vec![0; 24]);
        for (type_id, version, data) in &self.extra {
            file = file.chunk(*type_id, *version, data.clone());
        }
        file
    }

    /// Returns the bytes of the chunk file.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        self.file().bytes()
    }

    fn mesh_chunk(&self) -> Vec<u8> {
        let mut out = vec![0; MESH_LEN];
        write_u32(&mut out, 0, 1);
        write_u32(&mut out, 4, 0xC);
        write_u32(&mut out, 8, self.streamed_vertices);
        write_u32(&mut out, 0x5C, PHYSICS_ID);
        for &(slot, id) in &self.extra_slots {
            write_u32(&mut out, 0x5C + slot * 4, id);
        }
        out
    }

    fn physics_chunk(&self) -> Vec<u8> {
        let triangles = self.indices.len() / 3;
        let mut blob = vec![0; BLOB_VERTICES_AT];
        write_u32(&mut blob, BLOB_PRIMITIVE_AT, self.physics_primitive);
        write_u32(
            &mut blob,
            BLOB_COUNTS_AT,
            self.declared_vertices
                .unwrap_or_else(|| u32::try_from(self.vertices.len()).unwrap_or(u32::MAX)),
        );
        write_u32(
            &mut blob,
            BLOB_COUNTS_AT + 4,
            u32::try_from(triangles).unwrap_or(u32::MAX),
        );
        for vertex in &self.vertices {
            for component in vertex.position {
                blob.extend_from_slice(&component.to_le_bytes());
            }
        }
        for &index in &self.indices {
            blob.extend_from_slice(&u16::try_from(index).unwrap_or(u16::MAX).to_le_bytes());
        }
        blob.resize(blob.len() + BLOB_TREE_LEN, 0);

        let mut out = Vec::with_capacity(PHYSICS_HEADER_LEN + blob.len());
        put_u32(&mut out, u32::try_from(blob.len()).unwrap_or(u32::MAX));
        out.resize(PHYSICS_HEADER_LEN, 0);
        out.extend_from_slice(&blob);
        out
    }
}

fn write_u32(out: &mut [u8], at: usize, value: u32) {
    if let Some(slot) = out.get_mut(at..at + 4) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

fn mtl_name_chunk(library: &str) -> Vec<u8> {
    let mut out = vec![0; MTL_NAME_LEN];
    for (slot, byte) in out.iter_mut().zip(library.bytes()) {
        *slot = byte;
    }
    out
}

fn node_chunk(name: &str, object: u32, parent: u32, translation: [f32; 3]) -> Vec<u8> {
    let mut out = vec![0; NAME_LEN];
    for (slot, byte) in out.iter_mut().zip(name.bytes()) {
        *slot = byte;
    }
    for word in [object, parent, 0, MTL_NAME_ID, 0] {
        put_u32(&mut out, word);
    }
    let [x, y, z] = translation;
    let matrix = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [x, y, z, 0.0],
    ];
    for value in matrix.iter().flatten() {
        out.extend_from_slice(&f32::to_le_bytes(*value));
    }
    out.resize(out.len() + 40, 0);
    for _ in 0..3 {
        put_u32(&mut out, u32::MAX);
    }
    put_u32(&mut out, 0);
    out
}
