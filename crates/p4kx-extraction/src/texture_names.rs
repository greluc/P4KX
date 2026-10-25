// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use p4kx_texture::DdsHeader;

use crate::CUBE_FACE_NAMES;

const CUBE_FACES: u32 = 6;

/// Returns the file name suffix of every image of mip `level` in the order of its data, `_layerN`
/// for an array element, `_posx` … `_negz` for a cubemap face, `_sliceN` for a volume slice, in
/// that order; the suffix of a texture with one image is empty. With `cross`, a cubemap's six
/// faces are one image.
pub(crate) fn image_suffixes(header: &DdsHeader, level: u32, cross: bool) -> Vec<String> {
    let layers = header.array_size();
    let slices = header.slices(level);
    let faces = if cross && header.faces() == CUBE_FACES {
        1
    } else {
        header.faces()
    };
    let mut suffixes = Vec::new();
    for layer in 0..layers {
        for face in 0..faces {
            for slice in 0..slices {
                let mut suffix = String::new();
                if layers > 1 {
                    numbered(&mut suffix, "layer", layer, layers);
                }
                if faces > 1 {
                    match CUBE_FACE_NAMES.get(face as usize) {
                        Some(name) if header.faces() == CUBE_FACES => {
                            suffix.push('_');
                            suffix.push_str(name);
                        }
                        _ => numbered(&mut suffix, "face", face, faces),
                    }
                }
                if slices > 1 {
                    numbered(&mut suffix, "slice", slice, slices);
                }
                suffixes.push(suffix);
            }
        }
    }
    suffixes
}

fn numbered(suffix: &mut String, name: &str, index: u32, count: u32) {
    let width = count.saturating_sub(1).to_string().len();
    let _written = write!(suffix, "_{name}{index:0width$}");
}

/// Returns `out` with `suffix` appended to its file stem.
pub(crate) fn suffixed_path(out: &Path, suffix: &str) -> PathBuf {
    if suffix.is_empty() {
        return out.to_path_buf();
    }
    let mut name = OsString::from(out.file_stem().unwrap_or_default());
    name.push(suffix);
    if let Some(extension) = out.extension() {
        name.push(".");
        name.push(extension);
    }
    out.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use p4kx_testkit::{SplitDds, TestTextureFormat};

    use super::{image_suffixes, suffixed_path};
    use std::path::Path;

    fn header(dds: &SplitDds) -> p4kx_texture::DdsHeader {
        p4kx_texture::DdsHeader::parse(&dds.header_bytes()).expect("a valid header")
    }

    #[test]
    fn a_plain_texture_has_no_suffix() {
        let plain = header(&SplitDds::new(TestTextureFormat::R8, 8, 8));

        assert_eq!(image_suffixes(&plain, 0, false), [""]);
    }

    #[test]
    fn a_cubemap_has_a_suffix_per_face_or_none_as_a_cross() {
        let cube = header(&SplitDds::new(TestTextureFormat::BC7_SRGB, 8, 8).cubemap());

        assert_eq!(
            image_suffixes(&cube, 0, false),
            ["_posx", "_negx", "_posy", "_negy", "_posz", "_negz"]
        );
        assert_eq!(image_suffixes(&cube, 0, true), [""]);
    }

    #[test]
    fn array_elements_are_numbered_to_the_width_of_the_last() {
        let array = header(&SplitDds::new(TestTextureFormat::R8, 8, 8).array(12));

        let suffixes = image_suffixes(&array, 0, false);

        assert_eq!(suffixes.len(), 12);
        assert_eq!(suffixes.first().map(String::as_str), Some("_layer00"));
        assert_eq!(suffixes.last().map(String::as_str), Some("_layer11"));
    }

    #[test]
    fn volume_slices_halve_per_level() {
        let volume = header(&SplitDds::new(TestTextureFormat::R8, 8, 8).volume(4));

        assert_eq!(
            image_suffixes(&volume, 0, false),
            ["_slice0", "_slice1", "_slice2", "_slice3"]
        );
        assert_eq!(image_suffixes(&volume, 2, false), [""]);
    }

    #[test]
    fn a_cubemap_array_names_the_element_then_the_face() {
        let cubes = header(
            &SplitDds::new(TestTextureFormat::BC7_SRGB, 8, 8)
                .cubemap()
                .array(2),
        );

        let faces = image_suffixes(&cubes, 0, false);
        let crosses = image_suffixes(&cubes, 0, true);

        assert_eq!(faces.len(), 12);
        assert_eq!(faces.get(7).map(String::as_str), Some("_layer1_negx"));
        assert_eq!(crosses, ["_layer0", "_layer1"]);
    }

    #[test]
    fn the_suffix_goes_before_the_extension() {
        let out = Path::new("folder/texture.png");

        assert_eq!(
            suffixed_path(out, "_layer1"),
            Path::new("folder/texture_layer1.png")
        );
        assert_eq!(suffixed_path(out, ""), out);
    }
}
