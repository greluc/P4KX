// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use p4kx_model::{GREY_PROBE_MIP, MAX_GREY_SATURATION};

use crate::{PathIndex, Texture};

/// Returns the mean saturation of the probe mip of the texture a material file names as `file`.
///
/// The probe mip is [`GREY_PROBE_MIP`], or the last when the texture has fewer. `None` when the
/// Archive holds no such texture or it cannot be decoded.
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

/// The colour verdicts of the textures an [`PathIndex`] has judged, by the position of the
/// texture's Entry, so a map several materials or libraries name is decoded once.
#[derive(Debug, Default)]
pub(crate) struct GreyVerdicts(Mutex<HashMap<usize, Option<bool>>>);

impl GreyVerdicts {
    /// Returns the verdict kept for `position`, judging it with `judge` first when there is none.
    fn get_or_judge(&self, position: usize, judge: impl FnOnce() -> Option<bool>) -> Option<bool> {
        let kept = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&position)
            .copied();
        kept.unwrap_or_else(|| {
            let verdict = judge();
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(position, verdict);
            verdict
        })
    }
}

impl PathIndex<'_> {
    /// Returns whether the texture a material file names as `file` is grey by the criterion of
    /// [`p4kx_model::CompanionResolver::texture_is_grey`], or `None` when it cannot be judged. The
    /// verdict is kept for the lifetime of the index.
    pub(crate) fn texture_is_grey(&self, file: &str) -> Option<bool> {
        let entry = entry_path(file)?;
        let position = self.position(&entry)?;
        self.grey.get_or_judge(position, || {
            decode_saturation(self, &entry).map(|saturation| saturation <= MAX_GREY_SATURATION)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::error::Error;

    use p4kx_testkit::{ArchiveBuilder, SplitDds, TestEntry, TestTextureFormat, test_key};

    use super::GreyVerdicts;
    use crate::{Archive, PathIndex};

    type TestResult = Result<(), Box<dyn Error>>;

    const FILE: &str = "textures/mask_diff.tif";
    const ENTRY: &str = "Data\\Textures\\mask_diff.dds";
    const SIDE: u32 = 8;
    const GREY: [u8; 4] = [200, 200, 200, 255];
    const AT_LIMIT: [u8; 4] = [190, 190, 200, 255];
    const OVER_LIMIT: [u8; 4] = [189, 189, 200, 255];
    const RED: [u8; 4] = [0, 0, 255, 255];

    /// A BGRA8 DDS of `SIDE` squared with `mips` levels, level `n` filled with `texels[n]`.
    fn dds(texels: &[[u8; 4]]) -> Vec<u8> {
        let mips = u32::try_from(texels.len()).unwrap_or(1);
        let template = SplitDds::new(TestTextureFormat::LegacyBgra8, SIDE, SIDE).mips(mips);
        let mut bytes = template.header_bytes();
        bytes.truncate(template.header_len());
        for (level, texel) in texels.iter().enumerate() {
            let side = (SIDE >> level).max(1) as usize;
            bytes.extend(texel.iter().copied().cycle().take(side * side * 4));
        }
        bytes
    }

    fn verdict(texels: &[[u8; 4]]) -> Result<Option<bool>, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("Data.p4k");
        ArchiveBuilder::new()
            .entry(TestEntry::stored(ENTRY, dds(texels)))
            .write_to(&path)?;
        let archive = Archive::open_with_key(&path, Some(test_key()))?;
        Ok(PathIndex::new(&archive).texture_is_grey(FILE))
    }

    #[test]
    fn the_limit_itself_is_grey_and_a_little_above_it_is_not() -> TestResult {
        for (texel, expected) in [(GREY, true), (AT_LIMIT, true), (OVER_LIMIT, false)] {
            assert_eq!(
                verdict(&[RED, RED, RED, texel])?,
                Some(expected),
                "{texel:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn a_texture_with_fewer_than_four_mips_is_judged_by_its_last() -> TestResult {
        assert_eq!(verdict(&[RED, GREY])?, Some(true), "two mips, the last");
        assert_eq!(verdict(&[GREY, RED])?, Some(false), "two mips, the last");
        assert_eq!(verdict(&[GREY])?, Some(true), "one mip");
        Ok(())
    }

    #[test]
    fn a_missing_texture_cannot_be_judged() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("Data.p4k");
        ArchiveBuilder::new().write_to(&path)?;
        let archive = Archive::open_with_key(&path, Some(test_key()))?;
        assert_eq!(PathIndex::new(&archive).texture_is_grey(FILE), None);
        Ok(())
    }

    #[test]
    fn a_verdict_is_judged_once_per_texture() {
        let verdicts = GreyVerdicts::default();
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
