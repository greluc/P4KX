use std::path::Path;

use p4kx_model::{Animation, Dba};

use crate::{ChunkSource, ConversionError, UnreadAnimation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Caf,
    Dba,
}

pub(crate) fn kind(path: &str) -> Option<Kind> {
    let extension = Path::new(path).extension()?;
    if extension.eq_ignore_ascii_case("caf") {
        Some(Kind::Caf)
    } else if extension.eq_ignore_ascii_case("dba") {
        Some(Kind::Dba)
    } else {
        None
    }
}

pub(crate) struct Loaded {
    pub(crate) animations: Vec<Animation>,
    pub(crate) unread: Vec<UnreadAnimation>,
}

pub(crate) fn load(
    source: &ChunkSource,
    label: &str,
    kind: Kind,
) -> Result<Loaded, ConversionError> {
    let name = label.rsplit(['/', '\\']).next().unwrap_or(label);
    let failed = |source| ConversionError::Animation {
        file: label.to_owned(),
        source,
    };
    match kind {
        Kind::Caf => Ok(Loaded {
            animations: vec![Animation::read_caf(name, source.file()).map_err(failed)?],
            unread: Vec::new(),
        }),
        Kind::Dba => {
            let dba = Dba::read(name, source.file()).map_err(failed)?;
            let unread = dba
                .skipped
                .into_iter()
                .map(|skipped| UnreadAnimation {
                    source: label.to_owned(),
                    name: Some(skipped.name),
                    reason: skipped.reason.to_string(),
                })
                .collect();
            Ok(Loaded {
                animations: dba.animations,
                unread,
            })
        }
    }
}
