use super::patch::{Modification, apply_elems, apply_patch, find_modification};
use super::{VehicleImplementation, read};
use crate::markup::{self, Element};
use crate::{LoadoutError, LoadoutFileError, LoadoutFiles};

const DIRECTORY: &str = "Scripts/Entities/Vehicles/Implementations/Xml";

/// Reads the vehicle implementation an Entity names, with the Modification it selects.
///
/// The file is the Record's `vehicleDefinition`; when the Archive has none of that path, the
/// file of the same name in the implementation folder, then the file named after the Entity's
/// class name. The Modification's patch file is its `patchFile`, else
/// `Modifications/<definition name>_<modification>.xml`, else `Modifications/<class name>.xml`.
/// A patch file or Modification that cannot be found or read is an issue of the result; the
/// implementation without it is still returned.
pub(crate) fn load(
    files: &dyn LoadoutFiles,
    class_name: &str,
    definition: &str,
    modification: &str,
) -> Result<VehicleImplementation, LoadoutError> {
    let failed = |path: &str, reason: String| LoadoutError::VehicleImplementation {
        path: path.to_owned(),
        reason,
    };
    let stem = file_stem(definition);
    let mut candidates = Vec::new();
    push_unique(&mut candidates, definition);
    if !stem.is_empty() {
        push_unique(&mut candidates, &format!("{DIRECTORY}/{stem}.xml"));
    }
    push_unique(&mut candidates, &format!("{DIRECTORY}/{class_name}.xml"));
    let shown = if definition.is_empty() {
        class_name
    } else {
        definition
    };
    let (path, bytes) = read_first(files, &candidates)
        .map_err(|reason| failed(shown, reason))?
        .ok_or_else(|| failed(shown, LoadoutFileError::NotFound.to_string()))?;
    let mut root = markup::parse(&bytes).map_err(|reason| failed(&path, reason))?;
    if !root.name.eq_ignore_ascii_case("Vehicle") {
        return Err(failed(
            &path,
            format!("its root element is {}, not Vehicle", root.name),
        ));
    }
    let mut issues = Vec::new();
    let mut patch_path = None;
    let selected = (!modification.is_empty()).then(|| modification.to_owned());
    if let Some(name) = &selected {
        let found = find_modification(&root, name);
        patch_path = apply_modification(
            files,
            &mut root,
            found.as_ref(),
            (stem, class_name),
            name,
            &mut issues,
        );
        if found.is_none() && patch_path.is_none() {
            issues.push(failed(&path, format!("it names no Modification {name}")));
        }
    }
    let parts = read::parts(&root);
    Ok(VehicleImplementation {
        path,
        patch_path,
        modification: selected,
        name: root.attribute("name").unwrap_or_default().to_owned(),
        visibility: read::visibility(&parts),
        parts,
        issues,
    })
}

/// Applies the patch file and the `Elem` overrides of the Modification; returns the patch file
/// that was applied.
fn apply_modification(
    files: &dyn LoadoutFiles,
    root: &mut Element,
    found: Option<&Modification>,
    names: (&str, &str),
    modification: &str,
    issues: &mut Vec<LoadoutError>,
) -> Option<String> {
    let (stem, class_name) = names;
    let mut candidates = Vec::new();
    if let Some(file) = found.and_then(|found| found.patch_file.as_deref()) {
        push_unique(&mut candidates, &patch_path(file));
    }
    let explicit = candidates.len();
    if !stem.is_empty() {
        push_unique(
            &mut candidates,
            &format!("{DIRECTORY}/Modifications/{stem}_{modification}.xml"),
        );
    }
    push_unique(
        &mut candidates,
        &format!("{DIRECTORY}/Modifications/{class_name}.xml"),
    );
    let mut applied = None;
    match read_first(files, &candidates) {
        Ok(Some((path, bytes))) => match markup::parse(&bytes) {
            Ok(document) => {
                apply_patch(root, &document);
                applied = Some(path);
            }
            Err(reason) => issues.push(LoadoutError::VehicleImplementation { path, reason }),
        },
        Ok(None) => {
            if let Some(path) = candidates.first().filter(|_| explicit > 0) {
                issues.push(LoadoutError::VehicleImplementation {
                    path: path.clone(),
                    reason: LoadoutFileError::NotFound.to_string(),
                });
            }
        }
        Err(reason) => issues.push(LoadoutError::VehicleImplementation {
            path: candidates.first().cloned().unwrap_or_default(),
            reason,
        }),
    }
    if let Some(found) = found {
        apply_elems(root, &found.elems);
    }
    applied
}

/// Returns the first of `candidates` the files hold with its bytes, `None` when none exists,
/// or why a file that exists cannot be read.
fn read_first(
    files: &dyn LoadoutFiles,
    candidates: &[String],
) -> Result<Option<(String, Vec<u8>)>, String> {
    for path in candidates {
        match files.read(path) {
            Ok(bytes) => return Ok(Some((path.clone(), bytes))),
            Err(LoadoutFileError::NotFound) => {}
            Err(error) => return Err(format!("{path}: {error}")),
        }
    }
    Ok(None)
}

fn push_unique(list: &mut Vec<String>, path: &str) {
    let path = path.replace('\\', "/");
    if !path.is_empty() && !list.iter().any(|seen| seen.eq_ignore_ascii_case(&path)) {
        list.push(path);
    }
}

/// Returns the path of a patch file as the Archive holds it: relative to the implementation
/// folder unless it starts at `Scripts` or `Data`, and with `.xml` when it has no extension.
fn patch_path(file: &str) -> String {
    let file = file.trim().replace('\\', "/");
    let file = file.trim_start_matches('/');
    let rooted = ["scripts/", "data/"].iter().any(|root| {
        file.get(..root.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(root))
    });
    let mut path = if rooted {
        file.to_owned()
    } else {
        format!("{DIRECTORY}/{file}")
    };
    let has_extension = path
        .rsplit('/')
        .next()
        .is_some_and(|name| name.contains('.'));
    if !has_extension {
        path.push_str(".xml");
    }
    path
}

/// Returns the file name of `path` without its folders and extension.
fn file_stem(path: &str) -> &str {
    let name = path.rsplit(['/', '\\']).next().unwrap_or_default();
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}
