// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::elem::find_modification;
use super::patch::{ModificationReport, apply_elems, apply_patch};
use super::{ElemOverride, VehicleImplementation, read};
use crate::markup::{self, Budget, Element, MAX_ELEMENT_DEPTH};
use crate::{LoadoutError, LoadoutFileError, LoadoutFiles};

const DIRECTORY: &str = "Scripts/Entities/Vehicles/Implementations/Xml";

const LISTED: usize = 8;

fn issue(path: &str, reason: impl Into<String>) -> LoadoutError {
    LoadoutError::VehicleImplementation {
        path: path.to_owned(),
        reason: reason.into(),
    }
}

/// What selects the Modification: the implementation file, its name, the Entity's class name and
/// the Modification's name.
struct Selection<'a> {
    path: &'a str,
    stem: &'a str,
    class_name: &'a str,
    modification: &'a str,
}

/// Reads the vehicle implementation an Entity names, with the Modification it selects.
///
/// The file is the Record's `vehicleDefinition`; when the Archive has none of that path, the
/// file of the same name in the implementation folder, then the file named after the Entity's
/// class name. The Modification's patch file is its `patchFile`, else
/// `Modifications/<definition name>_<modification>.xml`, else `Modifications/<class name>.xml`.
/// Every path must lie below the implementation folder. A patch file or Modification that cannot
/// be found, read or applied within the budget is an issue of the result; the implementation
/// without it is still returned.
pub(crate) fn load(
    files: &dyn LoadoutFiles,
    class_name: &str,
    definition: &str,
    modification: &str,
) -> Result<VehicleImplementation, LoadoutError> {
    let stem = file_stem(definition);
    let candidates = definition_candidates(class_name, definition, stem);
    let shown = if definition.is_empty() {
        class_name
    } else {
        definition
    };
    let (path, bytes) = match read_first(files, &candidates) {
        Ok(Some(found)) => found,
        Ok(None) => {
            let reason = if candidates.is_empty() {
                format!("the path is not below {DIRECTORY}")
            } else {
                LoadoutFileError::NotFound.to_string()
            };
            return Err(issue(shown, reason));
        }
        Err(reason) => return Err(issue(shown, reason)),
    };
    let mut budget = Budget::new(bytes.len());
    let mut root = markup::parse(&bytes, &mut budget).map_err(|reason| issue(&path, reason))?;
    if !root.name.eq_ignore_ascii_case("Vehicle") {
        return Err(issue(
            &path,
            format!("its root element is {}, not Vehicle", root.name),
        ));
    }
    let mut issues = Vec::new();
    let mut patch_path = None;
    let mut report = None;
    let mut elems = Vec::new();
    let selected = (!modification.is_empty()).then(|| modification.to_owned());
    if let Some(name) = &selected {
        let selection = Selection {
            path: &path,
            stem,
            class_name,
            modification: name,
        };
        (patch_path, report, elems) =
            apply_modification(files, &mut root, &selection, &mut budget, &mut issues);
    }
    let parts = read::parts(&root);
    Ok(VehicleImplementation {
        structure: super::structure::digest(&root),
        path,
        patch_path,
        modification: selected,
        report,
        elems,
        name: root.attribute("name").unwrap_or_default().to_owned(),
        visibility: read::visibility(&parts),
        parts,
        issues,
    })
}

/// Returns the candidates for the implementation file: the definition, the file of its name in
/// the implementation folder and the file named after the class.
fn definition_candidates(class_name: &str, definition: &str, stem: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    add_candidate(&mut candidates, definition);
    if usable_name(stem) {
        add_candidate(&mut candidates, &format!("{DIRECTORY}/{stem}.xml"));
    }
    if usable_name(class_name) {
        add_candidate(&mut candidates, &format!("{DIRECTORY}/{class_name}.xml"));
    }
    candidates
}

/// Applies the patch file and the `Elem` overrides of the Modification; returns the patch file
/// that was applied, what the Modification did and the `Elem` overrides that took effect.
fn apply_modification(
    files: &dyn LoadoutFiles,
    root: &mut Element,
    selection: &Selection<'_>,
    budget: &mut Budget,
    issues: &mut Vec<LoadoutError>,
) -> (
    Option<String>,
    Option<ModificationReport>,
    Vec<ElemOverride>,
) {
    let mut taken = Vec::new();
    let found = find_modification(root, selection.modification);
    let patch_file = found.as_ref().and_then(|found| found.patch_file.as_deref());
    let (candidates, explicit) = patch_candidates(selection, patch_file, issues);
    let mut applied = None;
    let mut report = None;
    match read_first(files, &candidates) {
        Ok(Some((path, bytes))) => {
            if let Some(wanted) = explicit.as_ref().filter(|w| !w.eq_ignore_ascii_case(&path)) {
                issues.push(issue(wanted, LoadoutFileError::NotFound.to_string()));
            }
            budget.grow(bytes.len());
            match guarded(root, budget, |root, budget| {
                patch_stage(root, &bytes, budget)
            }) {
                Ok(done) => {
                    report_issues(&path, &done, issues);
                    applied = Some(path);
                    report = Some(done);
                }
                Err(reason) => issues.push(issue(&path, reason)),
            }
        }
        Ok(None) => {
            if let Some(wanted) = &explicit {
                issues.push(issue(wanted, LoadoutFileError::NotFound.to_string()));
            }
        }
        Err(reason) => issues.push(issue(selection.path, reason)),
    }
    match &found {
        Some(found) => match guarded(root, budget, |root, budget| {
            apply_elems(root, &found.elems, budget)
        }) {
            Ok(missed) => {
                taken = found
                    .elems
                    .iter()
                    .filter(|elem| !missed.contains(&elem.id_ref))
                    .cloned()
                    .collect();
                report
                    .get_or_insert_with(ModificationReport::default)
                    .unmatched_elems = missed;
            }
            Err(reason) => issues.push(issue(selection.path, reason)),
        },
        None if applied.is_none() => issues.push(issue(
            selection.path,
            format!("it names no Modification {}", selection.modification),
        )),
        None => {}
    }
    (applied, report, taken)
}

/// Returns the candidates for the patch file, and the one the Modification names itself.
fn patch_candidates(
    selection: &Selection<'_>,
    patch_file: Option<&str>,
    issues: &mut Vec<LoadoutError>,
) -> (Vec<String>, Option<String>) {
    let mut candidates = Vec::new();
    let mut explicit = None;
    if let Some(file) = patch_file {
        if let Some(path) = confine(&patch_path(file)) {
            push_unique(&mut candidates, &path);
            explicit = Some(path);
        } else {
            issues.push(issue(
                file,
                format!("the patch file is not below {DIRECTORY}"),
            ));
        }
    }
    if usable_name(selection.stem) && usable_name(selection.modification) {
        add_candidate(
            &mut candidates,
            &format!(
                "{DIRECTORY}/Modifications/{}_{}.xml",
                selection.stem, selection.modification
            ),
        );
    }
    if usable_name(selection.class_name) {
        add_candidate(
            &mut candidates,
            &format!("{DIRECTORY}/Modifications/{}.xml", selection.class_name),
        );
    }
    (candidates, explicit)
}

/// Parses the patch `bytes` and applies it to `root`, within `budget`.
fn patch_stage(
    root: &mut Element,
    bytes: &[u8],
    budget: &mut Budget,
) -> Result<ModificationReport, String> {
    let patch = markup::parse(bytes, budget)?;
    let done = apply_patch(root, &patch, budget)?;
    if markup::depth(root) > MAX_ELEMENT_DEPTH {
        return Err(markup::too_deep());
    }
    Ok(done)
}

/// Runs `run` on `root` and restores `root` as it was when `run` fails; the copy that makes this
/// possible must fit the budget, which it does not spend.
fn guarded<T>(
    root: &mut Element,
    budget: &mut Budget,
    run: impl FnOnce(&mut Element, &mut Budget) -> Result<T, String>,
) -> Result<T, String> {
    budget.check_copy(root)?;
    let backup = root.clone();
    run(&mut *root, budget).inspect_err(|_| *root = backup)
}

fn report_issues(path: &str, report: &ModificationReport, issues: &mut Vec<LoadoutError>) {
    let listed = |names: &[String]| {
        let shown: Vec<&str> = names.iter().take(LISTED).map(String::as_str).collect();
        let more = names.len().saturating_sub(LISTED);
        if more == 0 {
            shown.join(", ")
        } else {
            format!("{} and {more} more", shown.join(", "))
        }
    };
    if !report.unmatched_children.is_empty() {
        issues.push(issue(
            path,
            format!(
                "the patch replaces {} that the base lacks: {}",
                report.unmatched_children.len(),
                listed(&report.unmatched_children)
            ),
        ));
    }
    if !report.unmatched_ids.is_empty() {
        issues.push(issue(
            path,
            format!(
                "the patch names {} ids that no element has: {}",
                report.unmatched_ids.len(),
                listed(&report.unmatched_ids)
            ),
        ));
    }
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

/// Adds `path` to the candidates when it lies below the implementation folder.
fn add_candidate(list: &mut Vec<String>, path: &str) {
    if let Some(path) = confine(path) {
        push_unique(list, &path);
    }
}

fn push_unique(list: &mut Vec<String>, path: &str) {
    if !list.iter().any(|seen| seen.eq_ignore_ascii_case(path)) {
        list.push(path.to_owned());
    }
}

/// Returns `path` with `/` as separator and without a leading `Data`, or `None` when it has a
/// `.` or `..` segment or does not lie below the implementation folder.
fn confine(path: &str) -> Option<String> {
    let path = path.trim().replace('\\', "/");
    let mut segments: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    if segments
        .first()
        .is_some_and(|first| first.eq_ignore_ascii_case("data"))
    {
        segments.remove(0);
    }
    if segments
        .iter()
        .any(|part| matches!(*part, "." | "..") || part.contains(':'))
    {
        return None;
    }
    let root: Vec<&str> = DIRECTORY.split('/').collect();
    let inside = segments.len() > root.len()
        && segments
            .iter()
            .zip(&root)
            .all(|(part, wanted)| part.eq_ignore_ascii_case(wanted));
    inside.then(|| segments.join("/"))
}

/// Returns whether `name` can be a file name: it is not empty and holds no separator.
fn usable_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\', ':']) && !matches!(name, "." | "..")
}

/// Returns the path of a patch file: relative to the implementation folder unless it starts at
/// `Scripts` or `Data`, and with `.xml` when it has no extension.
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
