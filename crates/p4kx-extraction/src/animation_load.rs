// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Animation, AnimationError, AnimationHeader, Dba, DbaHeaders, SkippedAnimation};

use crate::{ChunkSource, ConversionError, UnreadAnimation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Caf,
    Dba,
}

pub(crate) fn kind(path: &str) -> Option<Kind> {
    let bytes = path.as_bytes();
    let extension = bytes.len().checked_sub(4).and_then(|at| bytes.get(at..))?;
    if extension.eq_ignore_ascii_case(b".caf") {
        Some(Kind::Caf)
    } else if extension.eq_ignore_ascii_case(b".dba") {
        Some(Kind::Dba)
    } else {
        None
    }
}

pub(crate) struct Loaded<T> {
    pub(crate) animations: Vec<T>,
    pub(crate) unread: Vec<UnreadAnimation>,
}

pub(crate) trait Described {
    fn name(&self) -> &str;
    fn path(&self) -> &str;
    fn additive(&self) -> bool;
}

impl Described for Animation {
    fn name(&self) -> &str {
        &self.name
    }
    fn path(&self) -> &str {
        &self.path
    }
    fn additive(&self) -> bool {
        self.additive
    }
}

impl Described for AnimationHeader {
    fn name(&self) -> &str {
        &self.name
    }
    fn path(&self) -> &str {
        &self.path
    }
    fn additive(&self) -> bool {
        self.additive
    }
}

fn skipped(label: &str, skipped: Vec<SkippedAnimation>) -> Vec<UnreadAnimation> {
    skipped
        .into_iter()
        .map(|skipped| UnreadAnimation {
            source: label.to_owned(),
            name: Some(skipped.name),
            reason: skipped.reason.to_string(),
        })
        .collect()
}

fn file_name(label: &str) -> &str {
    label.rsplit(['/', '\\']).next().unwrap_or(label)
}

fn failed(label: &str) -> impl Fn(AnimationError) -> ConversionError {
    move |source| ConversionError::Animation {
        file: label.to_owned(),
        source,
    }
}

pub(crate) fn load(
    source: &ChunkSource,
    label: &str,
    kind: Kind,
) -> Result<Loaded<Animation>, ConversionError> {
    let name = file_name(label);
    match kind {
        Kind::Caf => Ok(Loaded {
            animations: vec![Animation::read_caf(name, source.file()).map_err(failed(label))?],
            unread: Vec::new(),
        }),
        Kind::Dba => {
            let dba = Dba::read(name, source.file()).map_err(failed(label))?;
            Ok(Loaded {
                unread: skipped(label, dba.skipped),
                animations: dba.animations,
            })
        }
    }
}

pub(crate) fn load_headers(
    source: &ChunkSource,
    label: &str,
    kind: Kind,
) -> Result<Loaded<AnimationHeader>, ConversionError> {
    let name = file_name(label);
    match kind {
        Kind::Caf => Ok(Loaded {
            animations: vec![
                AnimationHeader::read_caf(name, source.file()).map_err(failed(label))?,
            ],
            unread: Vec::new(),
        }),
        Kind::Dba => {
            let dba = DbaHeaders::read(name, source.file()).map_err(failed(label))?;
            Ok(Loaded {
                unread: skipped(label, dba.skipped),
                animations: dba.animations,
            })
        }
    }
}
