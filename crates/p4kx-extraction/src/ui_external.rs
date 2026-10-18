use std::path::{Component, Path, PathBuf};

use p4kx_gfx::{ExternalImage, RgbaImage};

use crate::ui_movie::Origin;
use crate::{ConversionError, Texture, UiMovie};

const DDS: &str = "dds";

/// Decodes the external image `image` of the definition `id` from the DDS texture it names,
/// looked up relative to the movie in its Archive or on disk.
pub(crate) fn external_image(
    movie: &UiMovie<'_>,
    id: u16,
    image: &ExternalImage,
) -> Result<RgbaImage, ConversionError> {
    let file_name = if image.file_name.is_empty() {
        &image.export_name
    } else {
        &image.file_name
    };
    let mut tried = Vec::new();
    for candidate in candidates(file_name) {
        let texture = match movie.origin() {
            Origin::Entry(archive) => {
                let Some(path) = entry_path(movie.name(), &candidate) else {
                    continue;
                };
                let texture = Texture::from_archive(archive, &path);
                tried.push(path);
                texture
            }
            Origin::File(file) => {
                let Some(path) = file_path(file, &candidate) else {
                    continue;
                };
                let texture = Texture::from_file(&path);
                tried.push(path.display().to_string());
                texture
            }
        };
        let texture = match texture {
            Ok(texture) => texture,
            Err(ConversionError::UnknownEntry(_) | ConversionError::NoSuchFile { .. }) => continue,
            Err(error) => return Err(error),
        };
        let decoded = texture.decode_mip(0)?;
        let (width, height) = (decoded.width(), decoded.height());
        return RgbaImage::from_pixels(width, height, decoded.into_pixels())
            .map_err(|source| movie.asset_error(id, source));
    }
    Err(ConversionError::ExternalImageMissing {
        movie: movie.name().to_owned(),
        id,
        file: file_name.clone(),
        tried,
    })
}

/// Returns the file name as given and, unless it already ends in `.dds`, with the extension
/// `.dds` instead, as the game ships the textures the GFx exporter refers to.
fn candidates(file_name: &str) -> Vec<String> {
    let name_start = file_name.rfind(['/', '\\']).map_or(0, |at| at + 1);
    let (stem, extension) =
        file_name
            .rfind('.')
            .filter(|&at| at > name_start)
            .map_or((file_name, ""), |at| {
                (
                    file_name.get(..at).unwrap_or(file_name),
                    file_name.get(at + 1..).unwrap_or_default(),
                )
            });
    let mut names = vec![file_name.to_owned()];
    if !extension.eq_ignore_ascii_case(DDS) {
        names.push(format!("{stem}.{DDS}"));
    }
    names
}

/// Returns the Entry path `relative` names from the folder of the Entry `movie`, or `None` when
/// it is absolute or leaves the Archive.
pub(crate) fn entry_path(movie: &str, relative: &str) -> Option<String> {
    if relative.contains(':') || relative.starts_with(['/', '\\']) {
        return None;
    }
    let mut parts: Vec<&str> = movie.split(['/', '\\']).collect();
    parts.pop();
    for part in relative.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(parts.join("\\"))
}

/// Returns the file `relative` names from the folder of the movie file, or `None` when it is
/// absolute.
fn file_path(movie: &Path, relative: &str) -> Option<PathBuf> {
    if relative.starts_with(['/', '\\']) {
        return None;
    }
    let relative: PathBuf = relative.split(['/', '\\']).collect();
    let plain = relative.components().all(|component| {
        matches!(
            component,
            Component::Normal(_) | Component::CurDir | Component::ParentDir
        )
    });
    plain.then(|| {
        movie
            .parent()
            .map_or_else(|| relative.clone(), |folder| folder.join(&relative))
    })
}

#[cfg(test)]
mod tests {
    use super::{candidates, entry_path};

    #[test]
    fn the_dds_sibling_is_tried_after_the_named_file() {
        assert_eq!(
            candidates("../Textures/bg.png.tif"),
            ["../Textures/bg.png.tif", "../Textures/bg.png.dds"],
            "the last extension is replaced"
        );
        assert_eq!(
            candidates("Textures\\Button.DDS"),
            ["Textures\\Button.DDS"],
            "DDS"
        );
        assert_eq!(
            candidates("a.b/plain"),
            ["a.b/plain", "a.b/plain.dds"],
            "no extension"
        );
    }

    #[test]
    fn entry_paths_resolve_against_the_movie_folder_inside_the_archive() {
        assert_eq!(
            entry_path(
                "Data\\UI\\Screens\\assets\\SWF\\m.gfx",
                "../../../Textures/./x.dds"
            )
            .as_deref(),
            Some("Data\\UI\\Textures\\x.dds"),
            "dot segments"
        );
        assert_eq!(
            entry_path("Data\\UI\\m.gfx", "../../../x.dds"),
            None,
            "leaves"
        );
        assert_eq!(entry_path("Data\\UI\\m.gfx", "C:\\x.dds"), None, "drive");
        assert_eq!(entry_path("Data\\UI\\m.gfx", "/x.dds"), None, "absolute");
    }
}
