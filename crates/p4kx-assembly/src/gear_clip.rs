// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::AnimationChoice;
use p4kx_model::{Animation, Model};

use crate::cdf::{self, MAX_CDF_BYTES, is_character_definition};
use crate::markup::{self, Budget};
use crate::{ArchiveModels, ModelFailure, ModelProvider};

/// The most bytes of a skeleton's parameter file that are read.
const MAX_CHRPARAMS_BYTES: u64 = 1 << 20;

/// What a skeleton's parameter file (`.chrparams`) lists: the folder and database its animations
/// are stored in, and the animations by name and file.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ChrParams {
    pub(crate) folder: Option<String>,
    pub(crate) database: Option<String>,
    pub(crate) clips: Vec<(String, String)>,
}

/// Reads the parameter file in `bytes`.
///
/// # Errors
///
/// Returns why the document is no parameter file.
pub(crate) fn read_params(bytes: &[u8]) -> Result<ChrParams, String> {
    let mut budget = Budget::new(bytes.len());
    let root = markup::parse(bytes, &mut budget)?;
    if !root.name.eq_ignore_ascii_case("Params") {
        return Err(format!("the root element is {}, not Params", root.name));
    }
    let mut params = ChrParams {
        folder: None,
        database: None,
        clips: Vec::new(),
    };
    for list in root.children_named("AnimationList") {
        for clip in list.children_named("Animation") {
            let (Some(name), Some(file)) = (clip.attribute("name"), clip.attribute("path")) else {
                continue;
            };
            match name {
                "#filepath" => params.folder = Some(file.replace('\\', "/")),
                "$TracksDatabase" => params.database = Some(file.replace('\\', "/")),
                _ => params.clips.push((name.to_owned(), file.to_owned())),
            }
        }
    }
    Ok(params)
}

impl ChrParams {
    /// Returns the file of the clip that deploys the gear: the first whose name has `extend`,
    /// else the first with `deploy`, but never one that also names a retraction or compression.
    pub(crate) fn deploy_clip(&self) -> Option<&str> {
        ["extend", "deploy"].iter().find_map(|word| {
            self.clips.iter().find_map(|(name, file)| {
                let lower = name.to_ascii_lowercase();
                (lower.contains(word) && !lower.contains("retract") && !lower.contains("compress"))
                    .then_some(file.as_str())
            })
        })
    }
}

/// Returns `path` with the extension `.chrparams`.
fn params_path(skeleton: &str) -> String {
    let stem = skeleton.rsplit_once('.').map_or(skeleton, |(stem, _)| stem);
    format!("{stem}.chrparams")
}

impl ArchiveModels<'_> {
    pub(crate) fn pose_gear(
        &self,
        path: &str,
        model: &Model,
    ) -> Result<Option<Model>, ModelFailure> {
        let Some(entry) = self.entry_of(path) else {
            return Ok(None);
        };
        if !is_character_definition(&entry) {
            return Ok(None);
        }
        let unreadable = |reason: String| ModelFailure::Unreadable { reason };
        let bytes = self.file(&entry, MAX_CDF_BYTES)?;
        let character = cdf::read(&bytes).map_err(unreadable)?;
        let Some(skeleton) = character.skeleton else {
            return Ok(None);
        };
        let params = match self.file(&params_path(&skeleton), MAX_CHRPARAMS_BYTES) {
            Ok(bytes) => read_params(&bytes).map_err(unreadable)?,
            Err(ModelFailure::Missing) => return Ok(None),
            Err(other) => return Err(other),
        };
        let Some(clip) = params.deploy_clip() else {
            return Ok(None);
        };
        let animation = self.read_clip(&params, clip)?;
        crate::gear_pose::posed(model, &animation)
            .map(Some)
            .map_err(|failure| unreadable(failure.to_string()))
    }

    fn read_clip(&self, params: &ChrParams, clip: &str) -> Result<Animation, ModelFailure> {
        let extra: Vec<String> = params
            .database
            .iter()
            .filter_map(|database| self.entry_of(database))
            .chain(params.folder.iter().filter_map(|folder| {
                self.entry_of(&format!("{}/{clip}", folder.trim_end_matches('/')))
            }))
            .collect();
        if extra.is_empty() {
            return Err(ModelFailure::Unreadable {
                reason: "the animation database and the clip are not in the Archive".to_owned(),
            });
        }
        let choice = AnimationChoice::Named(vec![clip.to_owned()]);
        let set =
            self.animations_of(&extra, &choice)
                .map_err(|error| ModelFailure::Unreadable {
                    reason: error.to_string(),
                })?;
        set.animations
            .into_iter()
            .find(|animation| !animation.additive)
            .ok_or_else(|| ModelFailure::Unreadable {
                reason: format!("the clip {clip} is not in the animation database"),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{params_path, read_params};

    const PARAMS: &str = r##"<Params><AnimationList>
        <Animation name="#filepath" path="Animations\Ships\Gear"/>
        <Animation name="$TracksDatabase" path="Animations/Ships/Gear.dba"/>
        <Animation name="gear_compress" path="front_compress.caf"/>
        <Animation name="gear_extend" path="front_extend.caf"/>
        <Animation name="gear_retract" path="front_retract.caf"/>
        </AnimationList></Params>"##;

    #[test]
    fn the_extending_clip_is_the_one_that_deploys() {
        let params = read_params(PARAMS.as_bytes()).expect("params");
        assert_eq!(params.folder.as_deref(), Some("Animations/Ships/Gear"));
        assert_eq!(
            params.database.as_deref(),
            Some("Animations/Ships/Gear.dba")
        );
        assert_eq!(params.deploy_clip(), Some("front_extend.caf"));
    }

    #[test]
    fn deploy_is_taken_when_nothing_extends_and_compress_or_retract_never() {
        let text = r#"<Params><AnimationList>
            <Animation name="gear_retract" path="a_retract.caf"/>
            <Animation name="gear_compress" path="a_compress.caf"/>
            <Animation name="gear_deploy" path="a_deploy.caf"/>
            </AnimationList></Params>"#;
        assert_eq!(
            read_params(text.as_bytes()).expect("params").deploy_clip(),
            Some("a_deploy.caf")
        );
        let none = r#"<Params><AnimationList>
            <Animation name="gear_retract" path="a_retract.caf"/>
            </AnimationList></Params>"#;
        assert_eq!(
            read_params(none.as_bytes()).expect("params").deploy_clip(),
            None
        );
    }

    #[test]
    fn a_document_that_is_no_parameter_file_is_refused() {
        assert!(read_params(b"<Other/>").is_err());
        assert!(read_params(b"not xml").is_err());
    }

    #[test]
    fn the_parameter_file_sits_beside_the_skeleton() {
        assert_eq!(
            params_path("Objects/a/Gear_CHR.chr"),
            "Objects/a/Gear_CHR.chrparams"
        );
    }
}
