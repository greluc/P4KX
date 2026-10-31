// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use p4kx_model::{GREY_PROBE_MIP, MAX_GREY_SATURATION};

use crate::{PathIndex, Texture};

/// Returns the mean saturation of the probe mip of the texture a material file names as `file`.
///
/// This is the measuring half of [`p4kx_model::CompanionResolver::texture_is_grey`], exposed for
/// verification. The probe mip is [`GREY_PROBE_MIP`], or the last when the texture has fewer.
/// `None` when the Archive holds no such texture or it cannot be decoded.
#[doc(hidden)]
#[must_use]
pub fn probe_saturation(index: &PathIndex<'_>, file: &str) -> Option<f32> {
    let entry = entry_path(file)?;
    decode_saturation(index, &entry)
}

fn entry_path(file: &str) -> Option<String> {
    let path = p4kx_gltf::texture_path(file)?;
    Some(format!("Data\\{}", path.replace('/', "\\")))
}

fn decode_saturation(index: &PathIndex<'_>, entry: &str) -> Option<f32> {
    let texture = Texture::from_index(index, entry).ok()?;
    let last = texture.header().mip_count().saturating_sub(1);
    let image = texture.decode_mip(last.min(GREY_PROBE_MIP)).ok()?;
    Some(image.mean_saturation())
}

type Verdict = Arc<OnceLock<Option<bool>>>;

/// The colour verdicts of the textures judged while reading models.
///
/// They are kept by the position of the texture's Entry, so a map several materials or libraries
/// name is decoded once, also when threads ask for it at the same time.
///
/// The handle is opaque: pass it to [`crate::ModelSource`]. Positions are those of one Archive, so
/// use one handle per Archive, shared by every read of it.
#[derive(Debug, Default)]
pub struct TextureVerdicts(Mutex<HashMap<usize, Verdict>>);

impl TextureVerdicts {
    /// Returns an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether the texture a material file names as `file` is grey by the criterion of
    /// [`p4kx_model::CompanionResolver::texture_is_grey`], or `None` when it cannot be judged. The
    /// verdict is kept for the lifetime of the cache.
    pub(crate) fn is_grey(&self, index: &PathIndex<'_>, file: &str) -> Option<bool> {
        let entry = entry_path(file)?;
        let position = index.position(&entry)?;
        self.get_or_judge(position, || {
            decode_saturation(index, &entry).map(|saturation| saturation <= MAX_GREY_SATURATION)
        })
    }

    /// Returns the verdict of `position`, judging it with `judge` on the first ask only.
    fn get_or_judge(&self, position: usize, judge: impl FnOnce() -> Option<bool>) -> Option<bool> {
        let cell = Arc::clone(
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(position)
                .or_default(),
        );
        *cell.get_or_init(judge)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::error::Error;

    use p4kx_testkit::{ArchiveBuilder, SplitDds, TestEntry, TestTextureFormat, test_key};

    use super::TextureVerdicts;
    use crate::{Archive, PathIndex};

    type TestResult = Result<(), Box<dyn Error>>;

    const FILE: &str = "textures/mask_diff.tif";
    const ENTRY: &str = "Data\\Textures\\mask_diff.dds";
    const SIDE: u32 = 8;
    const GREY: [u8; 4] = [200, 200, 200, 255];
    // Saturation is (200 - low) / 200 in f64 narrowed to f32: 10 / 200 is the f64 nearest 0.05 and
    // narrows to the same f32 as the limit, so the first texel is exactly at it; 11 / 200 is above.
    const AT_LIMIT: [u8; 4] = [190, 190, 200, 255];
    const OVER_LIMIT: [u8; 4] = [189, 189, 200, 255];
    const RED: [u8; 4] = [0, 0, 255, 255];

    /// A BGRA8 DDS of `SIDE` squared with `N` mips, mip `n` filled with `texels[n]`.
    #[expect(clippy::cast_possible_truncation, reason = "N is at most 4")]
    fn dds<const N: usize>(texels: [[u8; 4]; N]) -> Vec<u8> {
        let template = SplitDds::new(TestTextureFormat::LegacyBgra8, SIDE, SIDE).mips(N as u32);
        let mut bytes = template.header_bytes();
        bytes.truncate(template.header_len());
        for (level, texel) in texels.iter().enumerate() {
            let side = (SIDE >> level).max(1) as usize;
            bytes.extend(texel.iter().copied().cycle().take(side * side * 4));
        }
        bytes
    }

    fn verdict<const N: usize>(texels: [[u8; 4]; N]) -> Result<Option<bool>, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("Data.p4k");
        ArchiveBuilder::new()
            .entry(TestEntry::stored(ENTRY, dds(texels)))
            .write_to(&path)?;
        let archive = Archive::open_with_key(&path, Some(test_key()))?;
        Ok(TextureVerdicts::new().is_grey(&PathIndex::new(&archive), FILE))
    }

    #[test]
    fn the_limit_itself_is_grey_and_a_little_above_it_is_not() -> TestResult {
        for (texel, expected) in [(GREY, true), (AT_LIMIT, true), (OVER_LIMIT, false)] {
            assert_eq!(
                verdict([RED, RED, RED, texel])?,
                Some(expected),
                "{texel:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn a_texture_with_fewer_than_four_mips_is_judged_by_its_last() -> TestResult {
        assert_eq!(verdict([RED, GREY])?, Some(true), "two mips, the last");
        assert_eq!(verdict([GREY, RED])?, Some(false), "two mips, the last");
        assert_eq!(verdict([GREY])?, Some(true), "one mip");
        Ok(())
    }

    #[test]
    fn a_missing_texture_cannot_be_judged() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("Data.p4k");
        ArchiveBuilder::new().write_to(&path)?;
        let archive = Archive::open_with_key(&path, Some(test_key()))?;
        let verdicts = TextureVerdicts::new();
        assert_eq!(verdicts.is_grey(&PathIndex::new(&archive), FILE), None);
        Ok(())
    }

    #[test]
    fn a_verdict_is_judged_once_per_texture() {
        let verdicts = TextureVerdicts::new();
        let judged = Cell::new(0);
        let judge = || {
            judged.set(judged.get() + 1);
            Some(true)
        };
        assert_eq!(verdicts.get_or_judge(7, judge), Some(true));
        assert_eq!(verdicts.get_or_judge(7, judge), Some(true));
        assert_eq!(judged.get(), 1, "the second call reads the kept verdict");
        assert_eq!(verdicts.get_or_judge(8, || None), None);
        assert_eq!(
            verdicts.get_or_judge(8, judge),
            None,
            "a failure is kept too"
        );
        assert_eq!(judged.get(), 1);
    }
}
