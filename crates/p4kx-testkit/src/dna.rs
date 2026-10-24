// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::bytes::{put_u16, put_u32, put_u64};

const MAGIC: &[u8; 16] = b"DNA V1.6\0\0\0\0\0\0\0\0";
const TABLE_AT: usize = 0x40;
const PAYLOAD_AT: usize = 0x140;
const NAME_LEN: usize = 40;

/// One control of the neutral morphs of a [`TestDna`]: how a code of a vertex turns into a move.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TestControl {
    /// The metres per code step on each axis.
    pub scale: [f32; 3],
    /// The metres of code 0 on each axis.
    pub bias: [f32; 3],
}

/// One blendshape cell of the expression morphs of a [`TestDna`].
#[derive(Debug, Clone, PartialEq)]
pub struct TestCell {
    /// The head.
    pub variant: usize,
    /// The blendshape.
    pub blendshape: usize,
    /// The vertex the first stride counts from.
    pub start_vertex: u16,
    /// The metres per code step on each axis.
    pub scale: [f32; 3],
    /// The metres of code 0 on each axis.
    pub bias: [f32; 3],
    /// The entries: a stride from the previous vertex and one code per axis.
    pub entries: Vec<(u8, [u8; 3])>,
}

/// One delta record of a [`TestDna`]: an output and its packed entries.
pub type TestDeltaRecord = (u16, Vec<u32>);

/// A synthetic DNA rig file, written with the layout of the real `.dna` files.
#[derive(Debug, Clone, PartialEq)]
pub struct TestDna {
    /// The identity in the header.
    pub identity: u64,
    /// The CRC-32C of the rig's name.
    pub rig_hash: u32,
    /// The names of the heads; the first is the base head.
    pub names: Vec<String>,
    /// The regions of the rig.
    pub regions: u8,
    /// The regions the file covers, as the file's region list; empty to cover all.
    pub covered: Vec<u32>,
    /// The vertices of the mesh.
    pub vertices: usize,
    /// The translations of each joint for each head, `[variant][joint]`.
    pub joints: Vec<Vec<[f32; 3]>>,
    /// The weights of each region for each joint, `[joint][region]`.
    pub joint_weights: Vec<Vec<f32>>,
    /// The weight of each covered region for each vertex, `[local region][vertex]`.
    pub masks: Vec<Vec<f32>>,
    /// The controls of the neutral morphs, head-major.
    pub controls: Vec<TestControl>,
    /// For each vertex the active controls, ascending, with the codes of each axis.
    pub codes: Vec<Vec<(usize, [u8; 3])>>,
    /// The blendshapes per head of the expression morphs.
    pub blendshapes: usize,
    /// The non-empty cells of the expression morphs.
    pub cells: Vec<TestCell>,
    /// The delta records of each head: main and secondary, each `(output, packed entries)`.
    pub deltas: Vec<[Vec<TestDeltaRecord>; 2]>,
    /// The main and secondary scale of each head's delta records.
    pub delta_scales: Vec<(f64, f64)>,
}

impl TestDna {
    /// A rig of the base head and two more, three regions of which the file covers two (regions 0
    /// and 2), two joints and four vertices.
    ///
    /// Head 1 moves vertex 0 and 1 in the first covered region, head 2 vertex 1 and 3 in the
    /// second; the masks split the vertices between the regions.
    #[must_use]
    pub fn small() -> Self {
        let control = |scale: f32, bias: f32| TestControl {
            scale: [scale; 3],
            bias: [bias; 3],
        };
        Self {
            identity: 0x1122_3344_5566_7788,
            rig_hash: 0xdd6c_67f6,
            names: ["base", "head_a", "head_b"]
                .map(str::to_owned)
                .into_iter()
                .collect(),
            regions: 3,
            covered: vec![0, 2],
            vertices: 4,
            joints: vec![
                vec![[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]],
                vec![[0.1, 0.0, 0.0], [1.0, 2.5, 3.0]],
                vec![[0.0, 0.2, 0.0], [1.0, 2.0, 3.5]],
            ],
            joint_weights: vec![vec![0.0; 3], vec![0.25, 0.25, 0.5]],
            masks: vec![vec![1.0, 0.5, 0.0, 0.0], vec![0.0, 0.5, 1.0, 1.0]],
            controls: vec![
                control(0.01, 0.0),
                control(0.02, 0.0),
                control(0.01, -0.5),
                control(0.5, 0.25),
            ],
            codes: vec![
                vec![(0, [10, 20, 30])],
                vec![(0, [1, 2, 3]), (3, [4, 5, 6])],
                vec![],
                vec![(3, [200, 100, 0])],
            ],
            blendshapes: 2,
            cells: vec![TestCell {
                variant: 1,
                blendshape: 1,
                start_vertex: 1,
                scale: [0.5, 0.5, 0.5],
                bias: [-1.0, 0.0, 1.0],
                entries: vec![(0, [2, 2, 2]), (2, [4, 0, 1])],
            }],
            deltas: vec![
                [Vec::new(), Vec::new()],
                [
                    vec![(3, vec![(2 << 9) | 7, (-3_i32 << 9).cast_unsigned() | 1])],
                    Vec::new(),
                ],
                [Vec::new(), vec![(5, vec![9])]],
            ],
            delta_scales: vec![(1.0, 2.0), (4_194_303.0, 8_388_606.0), (0.5, 0.25)],
        }
    }

    /// Returns the number of controls the neutral morphs have.
    #[must_use]
    pub fn control_count(&self) -> usize {
        let covered = if self.covered.is_empty() {
            usize::from(self.regions)
        } else {
            self.covered.len()
        };
        covered * self.names.len().saturating_sub(1)
    }

    /// Writes the file.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        let mut out = vec![0_u8; PAYLOAD_AT];
        let mut slots = [0_u64; 32];
        let mut place = |out: &mut Vec<u8>, slot: usize, data: Vec<u8>| {
            while !out.len().is_multiple_of(8) {
                out.push(0);
            }
            if let Some(entry) = slots.get_mut(slot) {
                *entry = out.len() as u64;
            }
            out.extend_from_slice(&data);
        };
        place(&mut out, 2, self.name_table());
        if !self.covered.is_empty() {
            place(&mut out, 12, self.region_list());
        }
        place(&mut out, 15, self.joint_table());
        place(&mut out, 21, self.weight_table());
        place(&mut out, 22, self.mask_table());
        place(&mut out, 18, self.neutral());
        place(&mut out, 19, self.expression());
        let deltas = self.delta_table(&out);
        place(&mut out, 31, deltas);
        self.write_header(&mut out, &slots);
        out
    }

    fn write_header(&self, out: &mut [u8], slots: &[u64; 32]) {
        let mut header = Vec::with_capacity(PAYLOAD_AT);
        header.extend_from_slice(MAGIC);
        put_u64(&mut header, self.identity);
        put_u32(&mut header, self.rig_hash);
        put_u32(&mut header, 0x1234_5678);
        header.resize(0x28, 0);
        header.push(self.regions);
        header.push(u8::try_from(self.covered.len()).unwrap_or(u8::MAX));
        put_u16(
            &mut header,
            u16::try_from(self.names.len()).unwrap_or(u16::MAX),
        );
        header.resize(0x30, 0);
        put_u32(
            &mut header,
            u32::try_from(self.vertices).unwrap_or(u32::MAX),
        );
        header.resize(TABLE_AT, 0);
        for slot in slots {
            put_u64(&mut header, *slot);
        }
        if let Some(target) = out.get_mut(..header.len()) {
            target.copy_from_slice(&header);
        }
    }

    fn name_table(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for name in &self.names {
            let mut entry = name.as_bytes().to_vec();
            entry.resize(NAME_LEN, 0);
            out.extend_from_slice(&entry);
        }
        out
    }

    fn region_list(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for region in &self.covered {
            put_u32(&mut out, *region);
        }
        out
    }

    fn joint_table(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for variant in &self.joints {
            for joint in variant {
                for value in joint {
                    out.extend_from_slice(&value.to_le_bytes());
                }
            }
        }
        out
    }

    fn weight_table(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for row in &self.joint_weights {
            for weight in row {
                out.extend_from_slice(&weight.to_le_bytes());
            }
        }
        put_u32(&mut out, 0);
        out
    }

    fn mask_table(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for region in &self.masks {
            for weight in region {
                out.extend_from_slice(&weight.to_le_bytes());
            }
        }
        put_u64(&mut out, 0);
        out
    }

    fn neutral(&self) -> Vec<u8> {
        let words = self.control_count().div_ceil(32);
        let mut out = Vec::new();
        for vertex in 0..self.vertices {
            let mut masks = vec![0_u32; words];
            for &(control, _) in self.codes.get(vertex).map_or(&[][..], Vec::as_slice) {
                if let Some(word) = masks.get_mut(control / 32) {
                    *word |= 1 << (control % 32);
                }
            }
            for word in masks {
                put_u32(&mut out, word);
            }
        }
        let mut cursor = 0_u32;
        for vertex in 0..self.vertices {
            put_u32(&mut out, cursor);
            cursor += u32::try_from(self.codes.get(vertex).map_or(0, Vec::len)).unwrap_or(0);
        }
        for control in &self.controls {
            for value in control.scale {
                out.extend_from_slice(&value.to_le_bytes());
            }
            put_u32(&mut out, 0);
            for value in control.bias {
                out.extend_from_slice(&value.to_le_bytes());
            }
            put_u32(&mut out, 0);
        }
        put_u32(&mut out, 0);
        for vertex in &self.codes {
            for (_, codes) in vertex {
                out.extend_from_slice(codes);
            }
        }
        out
    }

    fn expression(&self) -> Vec<u8> {
        let cells = self.names.len() * self.blendshapes;
        let table_len = cells * 32;
        let mut table = vec![0_u8; table_len];
        let mut blocks = Vec::new();
        for cell in &self.cells {
            let at = (cell.variant * self.blendshapes + cell.blendshape) * 32;
            let mut record = Vec::new();
            put_u32(
                &mut record,
                u32::try_from(table_len + blocks.len()).unwrap_or(0),
            );
            let count = u32::try_from(cell.entries.len()).unwrap_or(0);
            put_u32(&mut record, (u32::from(cell.start_vertex) << 16) | count);
            for value in cell.scale.iter().chain(&cell.bias) {
                record.extend_from_slice(&value.to_le_bytes());
            }
            if let Some(target) = table.get_mut(at..at + 32) {
                target.copy_from_slice(&record);
            }
            for (stride, codes) in &cell.entries {
                blocks.push(*stride);
                blocks.extend_from_slice(codes);
            }
        }
        if let Some(target) = table.get_mut(..4) {
            target.copy_from_slice(&u32::try_from(table_len).unwrap_or(0).to_le_bytes());
        }
        table.extend_from_slice(&blocks);
        table
    }

    fn delta_table(&self, before: &[u8]) -> Vec<u8> {
        let table_len = self.deltas.len() * 0x28;
        let base = (before.len().next_multiple_of(8) + table_len) as u64;
        let mut table = Vec::new();
        let mut payload = Vec::new();
        for (variant, sets) in self.deltas.iter().enumerate() {
            let (main_scale, secondary_scale) = self
                .delta_scales
                .get(variant)
                .copied()
                .unwrap_or((1.0, 1.0));
            let [main, secondary] = sets;
            let mut offsets = Vec::new();
            for records in [main, secondary] {
                offsets.push(base + payload.len() as u64);
                for (output, entries) in records {
                    put_u16(&mut payload, *output);
                    put_u16(&mut payload, u16::try_from(entries.len()).unwrap_or(0));
                    for packed in entries {
                        put_u32(&mut payload, *packed);
                    }
                }
            }
            put_u32(&mut table, u32::try_from(main.len()).unwrap_or(0));
            put_u32(&mut table, u32::try_from(secondary.len()).unwrap_or(0));
            table.extend_from_slice(&main_scale.to_le_bytes());
            table.extend_from_slice(&secondary_scale.to_le_bytes());
            for offset in offsets {
                put_u64(&mut table, offset);
            }
        }
        table.extend_from_slice(&payload);
        table
    }
}
