use ruzstd::encoding::{CompressionLevel, compress_to_vec};

use crate::bytes::{put_u16, put_u32, put_u64};

const CHF_SIZE: usize = 4096;
const HEADER_LEN: usize = 16;
const DNA_MAGIC: u32 = 0xfcd0_9394;
const GROUP_MARKER: u32 = 5;

/// A synthetic CHF character file, written with an uncompressed zstd frame so that a reader which
/// compresses anew cannot reproduce it by chance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestChf {
    /// The two header bytes behind the magic.
    pub flags: u16,
    /// The first inner version.
    pub female_version: u32,
    /// The second inner version; 8 and above end the body with a zero `u32`.
    pub male_version: u32,
    /// The stored bytes of the body type GUID.
    pub body_type: [u8; 16],
    /// The stored bytes of the voice GUID.
    pub voice: [u8; 16],
    /// The DNA head family and variant hashes; `None` writes an empty DNA block.
    pub dna_hashes: Option<(u32, u32)>,
    /// The DNA face parts, each a list of `(value, head id)` blends of equal length.
    pub face_parts: Vec<Vec<(u16, u16)>>,
    /// The root of the item port tree; `None` writes a port count of zero.
    pub ports: Option<TestItemPort>,
    /// The `u32` written behind the item port count.
    pub port_tail: u32,
    /// The material choices.
    pub materials: Vec<TestMaterial>,
    /// Bytes written behind the frame, cut where the file ends.
    pub padding: Vec<u8>,
    /// Replaces the CRC-32C in the header when set.
    pub crc_override: Option<u32>,
}

/// One port of a [`TestChf`] item tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestItemPort {
    /// Hash of the port name.
    pub port: u32,
    /// Stored bytes of the item GUID.
    pub item: [u8; 16],
    /// The binary layout's tail count.
    pub tail_count: u32,
    /// The ports of the item.
    pub children: Vec<Self>,
}

/// One material of a [`TestChf`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestMaterial {
    /// Hash of the attachment name.
    pub attachment: u32,
    /// Stored bytes of the base material GUID.
    pub base_material: [u8; 16],
    /// Material flags.
    pub flags: u32,
    /// The sub-materials.
    pub sub_materials: Vec<TestSubMaterial>,
}

/// One sub-material of a [`TestMaterial`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSubMaterial {
    /// Hash of the sub-material name.
    pub name: u32,
    /// `(slot, stored texture GUID)` pairs.
    pub textures: Vec<(u8, [u8; 16])>,
    /// `(name hash, value bits)` pairs.
    pub floats: Vec<(u32, u32)>,
    /// `(name hash, rgba)` pairs.
    pub colors: Vec<(u32, [u8; 4])>,
}

impl TestChf {
    /// A version 7 character with twelve face parts, a small item tree and two materials.
    #[must_use]
    pub fn v7() -> Self {
        Self::sample(7, 12)
    }

    /// A version 8 character with thirteen face parts, a small item tree and two materials.
    #[must_use]
    pub fn v8() -> Self {
        Self::sample(8, 13)
    }

    fn sample(male_version: u32, part_count: u16) -> Self {
        let face_parts = (0..part_count)
            .map(|part| {
                (0..4)
                    .map(|blend| (part * 1000 + blend, blend + 1))
                    .collect()
            })
            .collect();
        let port = |port, seed, tail_count, children| TestItemPort {
            port,
            item: [seed; 16],
            tail_count,
            children,
        };
        Self {
            flags: 0x127b,
            female_version: 2,
            male_version,
            body_type: [0x11; 16],
            voice: [0; 16],
            dna_hashes: Some((0x6d61_6c65, 0x7631_0001)),
            face_parts,
            port_tail: 0,
            ports: Some(port(
                0x1000,
                0xa0,
                2,
                vec![
                    port(0x2000, 0xa1, 0, Vec::new()),
                    port(0x3000, 0xa2, 2, vec![port(0x4000, 0xa3, 2, Vec::new())]),
                ],
            )),
            materials: sample_materials(),
            padding: b"\0\0\xef\xbe\xad\xde\xef\xbe\xad\xde".to_vec(),
            crc_override: None,
        }
    }

    /// A version 7 character file that holds only materials, without DNA and item ports, as the
    /// material presets of the game do.
    #[must_use]
    pub fn materials_only() -> Self {
        let sample = Self::v7();
        Self {
            dna_hashes: None,
            ports: None,
            port_tail: u32::try_from(sample.materials.len()).unwrap_or(u32::MAX),
            body_type: [0; 16],
            ..sample
        }
    }

    /// Returns the decompressed body.
    #[must_use]
    pub fn body(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_u32(&mut out, self.female_version);
        put_u32(&mut out, self.male_version);
        out.extend_from_slice(&self.body_type);
        out.extend_from_slice(&self.voice);
        self.put_dna(&mut out);
        put_u32(
            &mut out,
            u32::try_from(self.ports.as_ref().map_or(0, TestItemPort::count)).unwrap_or(u32::MAX),
        );
        put_u32(&mut out, self.port_tail);
        if let Some(ports) = &self.ports {
            ports.put(&mut out);
        }
        put_u32(&mut out, GROUP_MARKER);
        for (index, material) in self.materials.iter().enumerate() {
            material.put(&mut out, index + 1 == self.materials.len());
        }
        if self.male_version >= 8 {
            put_u32(&mut out, 0);
        }
        out
    }

    fn put_dna(&self, out: &mut Vec<u8>) {
        let Some((gender, variant)) = self.dna_hashes else {
            put_u64(out, 0);
            return;
        };
        let per_part = self.face_parts.first().map_or(4, Vec::len);
        let part_count = self.face_parts.len();
        put_u64(out, (24 + part_count * per_part * 4) as u64);
        put_u32(out, DNA_MAGIC);
        put_u32(out, gender);
        put_u32(out, variant);
        put_u32(out, 0);
        put_u16(out, u16::try_from(part_count).unwrap_or(u16::MAX));
        put_u16(out, u16::try_from(per_part).unwrap_or(u16::MAX));
        put_u16(out, 4);
        put_u16(out, 0x0123);
        for blend in 0..per_part {
            for part in &self.face_parts {
                let (value, head) = part.get(blend).copied().unwrap_or_default();
                put_u16(out, value);
                put_u16(out, head);
            }
        }
    }

    /// Returns the 4096-byte CHF file holding [`TestChf::body`] in an uncompressed zstd frame.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.with_body(&self.body())
    }

    /// Returns a valid CHF file around an arbitrary body, e.g. a corrupt one.
    #[must_use]
    pub fn with_body(&self, body: &[u8]) -> Vec<u8> {
        let frame = compress_to_vec(body, CompressionLevel::Uncompressed);
        let mut out = Vec::with_capacity(CHF_SIZE);
        put_u16(&mut out, 0x4242);
        put_u16(&mut out, self.flags);
        put_u32(&mut out, 0);
        put_u32(&mut out, u32::try_from(frame.len()).unwrap_or(u32::MAX));
        put_u32(&mut out, u32::try_from(body.len()).unwrap_or(u32::MAX));
        out.extend_from_slice(&frame);
        out.extend_from_slice(&self.padding);
        out.resize(CHF_SIZE, 0);
        let crc = self
            .crc_override
            .unwrap_or_else(|| crc32c::crc32c(out.get(HEADER_LEN..).unwrap_or_default()));
        if let Some(slot) = out.get_mut(4..8) {
            slot.copy_from_slice(&crc.to_le_bytes());
        }
        out
    }
}

impl TestItemPort {
    fn count(&self) -> u64 {
        1 + self.children.iter().map(Self::count).sum::<u64>()
    }

    fn put(&self, out: &mut Vec<u8>) {
        put_u32(out, self.port);
        out.extend_from_slice(&self.item);
        put_u32(out, u32::try_from(self.children.len()).unwrap_or(u32::MAX));
        put_u32(out, self.tail_count);
        for child in &self.children {
            child.put(out);
        }
    }
}

impl TestMaterial {
    fn put(&self, out: &mut Vec<u8>, last: bool) {
        put_u32(out, self.attachment);
        out.extend_from_slice(&self.base_material);
        put_u32(out, self.flags);
        out.extend_from_slice(&[0; 16]);
        put_u32(
            out,
            u32::try_from(self.sub_materials.len()).unwrap_or(u32::MAX),
        );
        put_u32(out, GROUP_MARKER);
        for (index, sub) in self.sub_materials.iter().enumerate() {
            sub.put(out);
            if !(last && index + 1 == self.sub_materials.len()) {
                put_u32(out, GROUP_MARKER);
            }
        }
    }
}

impl TestSubMaterial {
    fn put(&self, out: &mut Vec<u8>) {
        put_u32(out, self.name);
        put_u32(out, u32::try_from(self.textures.len()).unwrap_or(u32::MAX));
        for (slot, guid) in &self.textures {
            put_u32(out, 0);
            out.push(*slot);
            out.extend_from_slice(guid);
        }
        put_u64(out, self.floats.len() as u64);
        for (name, bits) in &self.floats {
            put_u32(out, *name);
            put_u32(out, *bits);
            put_u32(out, 0);
        }
        put_u64(out, self.colors.len() as u64);
        for (name, rgba) in &self.colors {
            put_u32(out, *name);
            out.extend_from_slice(rgba);
            put_u32(out, 0);
        }
    }
}

fn sample_materials() -> Vec<TestMaterial> {
    vec![
        TestMaterial {
            attachment: 0x0bad_cafe,
            base_material: [0x22; 16],
            flags: 0x0001_0203,
            sub_materials: vec![
                TestSubMaterial {
                    name: 0x5ab1,
                    textures: vec![(1, [0x33; 16])],
                    floats: vec![(0xf1, 0.25_f32.to_bits()), (0xf2, f32::NAN.to_bits())],
                    colors: vec![(0xc1, [10, 20, 30, 255])],
                },
                TestSubMaterial {
                    name: 0x5ab2,
                    textures: Vec::new(),
                    floats: vec![(0xf3, (-0.0_f32).to_bits())],
                    colors: Vec::new(),
                },
            ],
        },
        TestMaterial {
            attachment: 0x0dea_dbee,
            base_material: [0x44; 16],
            flags: 7,
            sub_materials: vec![TestSubMaterial {
                name: 0x5ab3,
                textures: vec![(0, [0x55; 16]), (3, [0x66; 16])],
                floats: Vec::new(),
                colors: vec![(0xc2, [1, 2, 3, 4])],
            }],
        },
    ]
}
