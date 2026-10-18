use p4kx_extraction::DdsHeader;

const CUBE_FACES: u32 = 6;
const CROSS_COLUMNS: u32 = 4;
const CROSS_ROWS: u32 = 3;

/// The most pixels the image of one mip may have to be shown, 8192 × 8192.
pub const MAX_PREVIEW_PIXELS: u64 = 8192 * 8192;

/// What the header of a texture says, as the image preview shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureInfo {
    format: String,
    width: u32,
    height: u32,
    mip_count: u32,
    faces: Option<u32>,
    decodable: bool,
    mip_labels: Vec<String>,
}

impl TextureInfo {
    /// Reads the format, size, mips and cubemap faces of `header`.
    pub(crate) fn of(header: &DdsHeader) -> Self {
        let cubemap = header.faces() == CUBE_FACES && header.array_size() == 1;
        Self::new(
            header.format().to_string(),
            [header.width(), header.height()],
            header.mip_count(),
            cubemap.then(|| header.faces()),
            header.format().is_decodable(),
        )
    }

    fn new(
        format: String,
        [width, height]: [u32; 2],
        mip_count: u32,
        faces: Option<u32>,
        decodable: bool,
    ) -> Self {
        let mip_labels = (0..mip_count)
            .map(|level| side(width.max(height), level).to_string())
            .collect();
        Self {
            format,
            width,
            height,
            mip_count,
            faces,
            decodable,
            mip_labels,
        }
    }

    /// Returns the name of the pixel format, such as `BC7_UNORM_SRGB`.
    #[must_use]
    pub fn format(&self) -> &str {
        &self.format
    }

    /// Returns the width of the largest mip in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height of the largest mip in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the number of mips.
    #[must_use]
    pub const fn mip_count(&self) -> u32 {
        self.mip_count
    }

    /// Returns the number of cubemap faces, or `None` when the texture is no cubemap.
    #[must_use]
    pub const fn cubemap_faces(&self) -> Option<u32> {
        self.faces
    }

    /// Returns whether the preview decodes the format.
    #[must_use]
    pub const fn is_decodable(&self) -> bool {
        self.decodable
    }

    /// Returns the labels of every mip, the largest first.
    #[must_use]
    pub fn mip_labels(&self) -> &[String] {
        &self.mip_labels
    }

    /// Returns the width and height of the image the preview shows for mip `level`: the mip, or
    /// a cubemap's faces laid out as a cross four faces wide and three high. `None` when the
    /// level does not exist.
    #[must_use]
    pub fn image_size(&self, level: u32) -> Option<[u32; 2]> {
        if level >= self.mip_count {
            return None;
        }
        let size = [side(self.width, level), side(self.height, level)];
        Some(match self.faces {
            Some(_) => [
                size[0].saturating_mul(CROSS_COLUMNS),
                size[1].saturating_mul(CROSS_ROWS),
            ],
            None => size,
        })
    }

    /// Returns the first level that is showable within `max_side` pixels a side, the largest mip
    /// the preview can show, or `None` when no level is small enough.
    #[must_use]
    pub fn first_showable_level(&self, max_side: u32) -> Option<u32> {
        (0..self.mip_count).find(|&level| self.is_showable(level, max_side))
    }

    /// Returns whether the image of `level` exists, has no side longer than `max_side` pixels
    /// and holds at most [`MAX_PREVIEW_PIXELS`].
    #[must_use]
    pub fn is_showable(&self, level: u32, max_side: u32) -> bool {
        let size = self.image_size(level);
        fits(size, [max_side, max_side])
            && size.is_some_and(|[w, h]| u64::from(w) * u64::from(h) <= MAX_PREVIEW_PIXELS)
    }

    /// Returns the largest showable mip that fits into `view` pixels, or the smallest mip when
    /// none fits; `None` when no level can be shown.
    #[must_use]
    pub fn fitting_level(&self, view: [u32; 2], max_side: u32) -> Option<u32> {
        let first = self.first_showable_level(max_side)?;
        (first..self.mip_count)
            .find(|&level| fits(self.image_size(level), view))
            .or_else(|| self.mip_count.checked_sub(1))
    }
}

fn side(length: u32, level: u32) -> u32 {
    length.checked_shr(level).unwrap_or(0).max(1)
}

fn fits(size: Option<[u32; 2]>, [width, height]: [u32; 2]) -> bool {
    size.is_some_and(|[w, h]| w <= width && h <= height)
}

#[cfg(test)]
mod tests {
    use super::TextureInfo;

    fn texture(size: [u32; 2], mips: u32, faces: Option<u32>) -> TextureInfo {
        TextureInfo::new("BC1_UNORM".to_owned(), size, mips, faces, true)
    }

    #[test]
    fn the_largest_mip_that_fits_the_view_is_chosen() {
        let info = texture([1024, 512], 11, None);

        assert_eq!(info.fitting_level([300, 300], 8192), Some(2), "256 × 128");
        assert_eq!(info.fitting_level([1024, 512], 8192), Some(0), "exactly");
        assert_eq!(info.fitting_level([1023, 4096], 8192), Some(1), "narrower");
    }

    #[test]
    fn the_smallest_mip_is_chosen_when_none_fits() {
        let info = texture([64, 64], 3, None);

        assert_eq!(
            info.fitting_level([4, 4], 8192),
            Some(2),
            "16 × 16 is the smallest"
        );
    }

    #[test]
    fn mips_larger_than_the_window_can_show_are_never_chosen() {
        let info = texture([4096, 4096], 13, None);

        assert_eq!(info.first_showable_level(2048), Some(1), "2048");
        assert_eq!(info.fitting_level([8192, 8192], 2048), Some(1), "capped");
    }

    #[test]
    fn a_cubemap_is_measured_as_its_cross() {
        let info = texture([128, 128], 8, Some(6));

        assert_eq!(info.image_size(1), Some([256, 192]), "four by three faces");
        assert_eq!(
            info.fitting_level([300, 200], 8192),
            Some(1),
            "the cross fits"
        );
    }

    #[test]
    fn mips_are_labelled_by_their_longer_side() {
        let info = texture([8, 2], 4, None);

        assert_eq!(info.mip_labels(), ["8", "4", "2", "1"], "labels");
        assert_eq!(info.image_size(3), Some([1, 1]), "never below one pixel");
        assert_eq!(info.image_size(4), None, "no such level");
    }

    #[test]
    fn images_above_the_pixel_cap_are_not_shown_whatever_the_renderer() {
        let info = texture([16_384, 16_384], 15, None);

        assert!(!info.is_showable(0, u32::MAX), "16384 × 16384");
        assert_eq!(info.first_showable_level(u32::MAX), Some(1), "8192 × 8192");
    }

    #[test]
    fn a_texture_without_mips_shows_nothing() {
        let info = texture([8, 8], 0, None);

        assert_eq!(
            info.fitting_level([100, 100], 8192),
            None,
            "nothing to show"
        );
    }
}
