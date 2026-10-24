// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// The red, green and blue of endpoint W, the first of region 0.
const RW: u8 = 0;
const GW: u8 = 1;
const BW: u8 = 2;
/// The red, green and blue of endpoint X, the second of region 0.
const RX: u8 = 3;
const GX: u8 = 4;
const BX: u8 = 5;
/// The red, green and blue of endpoint Y, the first of region 1.
const RY: u8 = 6;
const GY: u8 = 7;
const BY: u8 = 8;
/// The red, green and blue of endpoint Z, the second of region 1.
const RZ: u8 = 9;
const GZ: u8 = 10;
const BZ: u8 = 11;
/// The partition number.
pub(crate) const PARTITION: u8 = 12;
/// The number of header fields: twelve endpoint channels and the partition.
pub(crate) const FIELDS: usize = 13;

/// `len` bits of the block that go to bits `shift` upwards of the header field `field`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Run {
    pub(crate) field: u8,
    pub(crate) shift: u8,
    pub(crate) len: u8,
}

const fn run(field: u8, shift: u8, len: u8) -> Run {
    Run { field, shift, len }
}

const fn bit(field: u8, at: u8) -> Run {
    run(field, at, 1)
}

/// The layout of one BC6H mode.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Mode {
    /// Bits of the mode number that start the block.
    pub(crate) number_bits: u32,
    /// Two regions with a partition and 3-bit indices, or one region with 4-bit indices.
    pub(crate) two_regions: bool,
    /// Whether X, Y and Z are stored as deltas from W.
    pub(crate) transformed: bool,
    /// Precision of the endpoints after the transform.
    pub(crate) endpoint_bits: u32,
    /// Stored bits of X, Y and Z per red, green and blue.
    pub(crate) delta_bits: [u32; 3],
    /// Where the header fields are stored, in block order after the mode number.
    pub(crate) runs: &'static [Run],
}

const fn two(endpoint_bits: u32, delta_bits: [u32; 3], runs: &'static [Run]) -> Mode {
    Mode {
        number_bits: 5,
        two_regions: true,
        transformed: true,
        endpoint_bits,
        delta_bits,
        runs,
    }
}

const fn one(endpoint_bits: u32, delta_bits: u32, runs: &'static [Run]) -> Mode {
    Mode {
        number_bits: 5,
        two_regions: false,
        transformed: true,
        endpoint_bits,
        delta_bits: [delta_bits; 3],
        runs,
    }
}

const D: Run = run(PARTITION, 0, 5);

/// Modes 1 to 14 of the D3D11 functional specification.
#[rustfmt::skip]
const MODES: [Mode; 14] = [
    Mode {
        number_bits: 2,
        ..two(10, [5; 3], &[
            bit(GY, 4), bit(BY, 4), bit(BZ, 4), run(RW, 0, 10), run(GW, 0, 10), run(BW, 0, 10),
            run(RX, 0, 5), bit(GZ, 4), run(GY, 0, 4), run(GX, 0, 5), bit(BZ, 0), run(GZ, 0, 4),
            run(BX, 0, 5), bit(BZ, 1), run(BY, 0, 4), run(RY, 0, 5), bit(BZ, 2), run(RZ, 0, 5),
            bit(BZ, 3), D,
        ])
    },
    Mode {
        number_bits: 2,
        ..two(7, [6; 3], &[
            bit(GY, 5), bit(GZ, 4), bit(GZ, 5), run(RW, 0, 7), bit(BZ, 0), bit(BZ, 1), bit(BY, 4),
            run(GW, 0, 7), bit(BY, 5), bit(BZ, 2), bit(GY, 4), run(BW, 0, 7), bit(BZ, 3),
            bit(BZ, 5), bit(BZ, 4), run(RX, 0, 6), run(GY, 0, 4), run(GX, 0, 6), run(GZ, 0, 4),
            run(BX, 0, 6), run(BY, 0, 4), run(RY, 0, 6), run(RZ, 0, 6), D,
        ])
    },
    two(11, [5, 4, 4], &[
        run(RW, 0, 10), run(GW, 0, 10), run(BW, 0, 10), run(RX, 0, 5), bit(RW, 10),
        run(GY, 0, 4), run(GX, 0, 4), bit(GW, 10), bit(BZ, 0), run(GZ, 0, 4), run(BX, 0, 4),
        bit(BW, 10), bit(BZ, 1), run(BY, 0, 4), run(RY, 0, 5), bit(BZ, 2), run(RZ, 0, 5),
        bit(BZ, 3), D,
    ]),
    two(11, [4, 5, 4], &[
        run(RW, 0, 10), run(GW, 0, 10), run(BW, 0, 10), run(RX, 0, 4), bit(RW, 10), bit(GZ, 4),
        run(GY, 0, 4), run(GX, 0, 5), bit(GW, 10), run(GZ, 0, 4), run(BX, 0, 4), bit(BW, 10),
        bit(BZ, 1), run(BY, 0, 4), run(RY, 0, 4), bit(BZ, 0), bit(BZ, 2), run(RZ, 0, 4),
        bit(GY, 4), bit(BZ, 3), D,
    ]),
    two(11, [4, 4, 5], &[
        run(RW, 0, 10), run(GW, 0, 10), run(BW, 0, 10), run(RX, 0, 4), bit(RW, 10), bit(BY, 4),
        run(GY, 0, 4), run(GX, 0, 4), bit(GW, 10), bit(BZ, 0), run(GZ, 0, 4), run(BX, 0, 5),
        bit(BW, 10), run(BY, 0, 4), run(RY, 0, 4), bit(BZ, 1), bit(BZ, 2), run(RZ, 0, 4),
        bit(BZ, 4), bit(BZ, 3), D,
    ]),
    two(9, [5; 3], &[
        run(RW, 0, 9), bit(BY, 4), run(GW, 0, 9), bit(GY, 4), run(BW, 0, 9), bit(BZ, 4),
        run(RX, 0, 5), bit(GZ, 4), run(GY, 0, 4), run(GX, 0, 5), bit(BZ, 0), run(GZ, 0, 4),
        run(BX, 0, 5), bit(BZ, 1), run(BY, 0, 4), run(RY, 0, 5), bit(BZ, 2), run(RZ, 0, 5),
        bit(BZ, 3), D,
    ]),
    two(8, [6, 5, 5], &[
        run(RW, 0, 8), bit(GZ, 4), bit(BY, 4), run(GW, 0, 8), bit(BZ, 2), bit(GY, 4),
        run(BW, 0, 8), bit(BZ, 3), bit(BZ, 4), run(RX, 0, 6), run(GY, 0, 4), run(GX, 0, 5),
        bit(BZ, 0), run(GZ, 0, 4), run(BX, 0, 5), bit(BZ, 1), run(BY, 0, 4), run(RY, 0, 6),
        run(RZ, 0, 6), D,
    ]),
    two(8, [5, 6, 5], &[
        run(RW, 0, 8), bit(BZ, 0), bit(BY, 4), run(GW, 0, 8), bit(GY, 5), bit(GY, 4),
        run(BW, 0, 8), bit(GZ, 5), bit(BZ, 4), run(RX, 0, 5), bit(GZ, 4), run(GY, 0, 4),
        run(GX, 0, 6), run(GZ, 0, 4), run(BX, 0, 5), bit(BZ, 1), run(BY, 0, 4), run(RY, 0, 5),
        bit(BZ, 2), run(RZ, 0, 5), bit(BZ, 3), D,
    ]),
    two(8, [5, 5, 6], &[
        run(RW, 0, 8), bit(BZ, 1), bit(BY, 4), run(GW, 0, 8), bit(BY, 5), bit(GY, 4),
        run(BW, 0, 8), bit(BZ, 5), bit(BZ, 4), run(RX, 0, 5), bit(GZ, 4), run(GY, 0, 4),
        run(GX, 0, 5), bit(BZ, 0), run(GZ, 0, 4), run(BX, 0, 6), run(BY, 0, 4), run(RY, 0, 5),
        bit(BZ, 2), run(RZ, 0, 5), bit(BZ, 3), D,
    ]),
    Mode {
        transformed: false,
        ..two(6, [6; 3], &[
            run(RW, 0, 6), bit(GZ, 4), bit(BZ, 0), bit(BZ, 1), bit(BY, 4), run(GW, 0, 6),
            bit(GY, 5), bit(BY, 5), bit(BZ, 2), bit(GY, 4), run(BW, 0, 6), bit(GZ, 5),
            bit(BZ, 3), bit(BZ, 5), bit(BZ, 4), run(RX, 0, 6), run(GY, 0, 4), run(GX, 0, 6),
            run(GZ, 0, 4), run(BX, 0, 6), run(BY, 0, 4), run(RY, 0, 6), run(RZ, 0, 6), D,
        ])
    },
    Mode {
        transformed: false,
        ..one(10, 10, &[
            run(RW, 0, 10), run(GW, 0, 10), run(BW, 0, 10), run(RX, 0, 10), run(GX, 0, 10),
            run(BX, 0, 10),
        ])
    },
    one(11, 9, &[
        run(RW, 0, 10), run(GW, 0, 10), run(BW, 0, 10), run(RX, 0, 9), bit(RW, 10),
        run(GX, 0, 9), bit(GW, 10), run(BX, 0, 9), bit(BW, 10),
    ]),
    one(12, 8, &[
        run(RW, 0, 10), run(GW, 0, 10), run(BW, 0, 10), run(RX, 0, 8), bit(RW, 11), bit(RW, 10),
        run(GX, 0, 8), bit(GW, 11), bit(GW, 10), run(BX, 0, 8), bit(BW, 11), bit(BW, 10),
    ]),
    one(16, 4, &[
        run(RW, 0, 10), run(GW, 0, 10), run(BW, 0, 10), run(RX, 0, 4), bit(RW, 15), bit(RW, 14),
        bit(RW, 13), bit(RW, 12), bit(RW, 11), bit(RW, 10), run(GX, 0, 4), bit(GW, 15),
        bit(GW, 14), bit(GW, 13), bit(GW, 12), bit(GW, 11), bit(GW, 10), run(BX, 0, 4),
        bit(BW, 15), bit(BW, 14), bit(BW, 13), bit(BW, 12), bit(BW, 11), bit(BW, 10),
    ]),
];

/// The mode of each value of the first five bits of a block; `None` for the reserved ones.
const MODE_OF: [Option<u8>; 32] = mode_numbers();

#[expect(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    reason = "compile-time evaluation within the fixed table sizes, where a slip fails the build"
)]
const fn mode_numbers() -> [Option<u8>; 32] {
    let five_bit: [(u8, u8); 12] = [
        (0b00010, 2),
        (0b00110, 3),
        (0b01010, 4),
        (0b01110, 5),
        (0b10010, 6),
        (0b10110, 7),
        (0b11010, 8),
        (0b11110, 9),
        (0b00011, 10),
        (0b00111, 11),
        (0b01011, 12),
        (0b01111, 13),
    ];
    let mut out = [None; 32];
    let mut value = 0;
    while value < 32 {
        if value & 0b10 == 0 {
            out[value] = Some((value & 1) as u8);
        }
        value += 1;
    }
    let mut entry = 0;
    while entry < five_bit.len() {
        let (value, mode) = five_bit[entry];
        out[value as usize] = Some(mode);
        entry += 1;
    }
    out
}

/// Returns the mode of a block from its first five bits, or `None` for a reserved mode.
pub(crate) fn mode_of(first_bits: u128) -> Option<&'static Mode> {
    let value = usize::try_from(first_bits & 0x1F).ok()?;
    let number = (*MODE_OF.get(value)?)?;
    MODES.get(usize::from(number))
}
