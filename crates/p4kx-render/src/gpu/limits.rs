// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use thiserror::Error;

/// The longest side, in texels, of a texture the viewer uploads, whatever the device allows.
pub const MAX_TEXTURE_SIDE: u32 = 8192;

/// The granularity, in pixels, the render target grows in, so resizing the panel does not
/// allocate a texture for every pixel of change.
pub(super) const TARGET_STEP: u32 = 64;

/// Why a resource is not uploaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Rejected {
    /// A texture has a side of zero or above the limit.
    #[error("a texture of {width} × {height} texels exceeds the side limit {limit}")]
    TextureSide {
        /// The width in texels.
        width: u32,
        /// The height in texels.
        height: u32,
        /// The longest side allowed.
        limit: u32,
    },
    /// A texture's pixel data does not have four bytes per texel.
    #[error("a texture of {width} × {height} texels does not match {bytes} bytes of data")]
    TextureData {
        /// The width in texels.
        width: u32,
        /// The height in texels.
        height: u32,
        /// The number of bytes given.
        bytes: usize,
    },
    /// A buffer is larger than the device allows.
    #[error("a buffer of {bytes} bytes exceeds the device limit {limit}")]
    Buffer {
        /// The size asked for.
        bytes: u64,
        /// The largest buffer the device creates.
        limit: u64,
    },
}

/// Checks that `data_len` bytes are the RGBA texels of a `width` × `height` texture whose
/// sides are at most `device_limit` and [`MAX_TEXTURE_SIDE`].
///
/// # Errors
///
/// Returns [`Rejected::TextureSide`] for a side of zero or above the limit and
/// [`Rejected::TextureData`] when the data is not four bytes per texel.
pub fn check_texture(
    width: u32,
    height: u32,
    data_len: usize,
    device_limit: u32,
) -> Result<(), Rejected> {
    let limit = device_limit.min(MAX_TEXTURE_SIDE);
    if width == 0 || height == 0 || width > limit || height > limit {
        return Err(Rejected::TextureSide {
            width,
            height,
            limit,
        });
    }
    let expected = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|texels| texels.checked_mul(4));
    if expected == u64::try_from(data_len).ok() {
        Ok(())
    } else {
        Err(Rejected::TextureData {
            width,
            height,
            bytes: data_len,
        })
    }
}

/// Checks that a buffer of `bytes` is at most `device_limit`.
///
/// # Errors
///
/// Returns [`Rejected::Buffer`] when it is larger.
pub const fn check_buffer(bytes: u64, device_limit: u64) -> Result<(), Rejected> {
    if bytes > device_limit {
        Err(Rejected::Buffer {
            bytes,
            limit: device_limit,
        })
    } else {
        Ok(())
    }
}

/// Returns the size of the render target for a view of `size` pixels: at least one pixel, grown
/// to the next multiple of [`TARGET_STEP`] and at most `device_limit` a side.
#[must_use]
pub fn target_extent(size: [u32; 2], device_limit: u32) -> [u32; 2] {
    size.map(|side| {
        let stepped = side
            .max(1)
            .div_ceil(TARGET_STEP)
            .saturating_mul(TARGET_STEP);
        stepped.min(device_limit.max(1))
    })
}

#[cfg(test)]
mod tests {
    use super::{Rejected, check_buffer, check_texture, target_extent};

    #[test]
    fn a_texture_within_the_limits_with_four_bytes_per_texel_passes() {
        assert_eq!(check_texture(2, 3, 24, 8192), Ok(()), "2 × 3 RGBA");
    }

    #[test]
    fn a_texture_side_of_zero_or_above_the_smaller_limit_is_rejected() {
        let side = |width, height, limit| Rejected::TextureSide {
            width,
            height,
            limit,
        };

        assert_eq!(check_texture(0, 4, 0, 8192), Err(side(0, 4, 8192)), "zero");
        assert_eq!(
            check_texture(9000, 1, 36_000, 16_384),
            Err(side(9000, 1, 8192)),
            "above the viewer limit"
        );
        assert_eq!(
            check_texture(4096, 1, 16_384, 2048),
            Err(side(4096, 1, 2048)),
            "above the device limit"
        );
    }

    #[test]
    fn texture_data_of_the_wrong_length_is_rejected_without_overflow() {
        let data = |width, height, bytes| Rejected::TextureData {
            width,
            height,
            bytes,
        };

        assert_eq!(check_texture(2, 2, 15, 8192), Err(data(2, 2, 15)), "short");
        assert_eq!(check_texture(2, 2, 17, 8192), Err(data(2, 2, 17)), "long");
        assert!(
            check_texture(8192, 8192, usize::MAX, 8192).is_err(),
            "a huge length does not wrap"
        );
    }

    #[test]
    fn a_buffer_above_the_device_limit_is_rejected() {
        assert_eq!(check_buffer(100, 100), Ok(()), "exactly the limit");
        assert_eq!(
            check_buffer(101, 100),
            Err(Rejected::Buffer {
                bytes: 101,
                limit: 100
            }),
            "one over"
        );
    }

    #[test]
    fn the_target_grows_in_steps_and_stays_within_the_device() {
        assert_eq!(target_extent([0, 0], 8192), [64, 64], "never empty");
        assert_eq!(target_extent([64, 65], 8192), [64, 128], "next multiple");
        assert_eq!(target_extent([1000, 700], 8192), [1024, 704], "stepped");
        assert_eq!(target_extent([100_000, 5], 4096), [4096, 64], "clamped");
    }
}
