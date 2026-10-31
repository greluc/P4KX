// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{MAX_PART_BYTES, NameRule, RenameCause};

use super::Texts;

/// A text about the Records the file-name rule renamed, shared by the Extraction summary and the
/// Record export summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenameText {
    /// The heading of the list of renamed Records; takes `number`.
    Heading,
    /// One renamed Record; takes `path`, `guid` and `reason`.
    Line,
    /// The Record name breaks a rule; takes `rule`.
    Name,
    /// A part of the record path breaks a rule; takes `rule`.
    Path,
    /// The part is empty.
    RuleEmpty,
    /// The part is `.` or `..`.
    RuleDotPart,
    /// The part holds a forbidden character; takes `char`.
    RuleForbiddenCharacter,
    /// The part ends with a dot or a space.
    RuleTrailingDotOrSpace,
    /// The part is a Windows device name.
    RuleDeviceName,
    /// The part is too long; takes `bytes` and `max`.
    RuleTooManyBytes,
}

impl RenameText {
    /// Every text, for checks of the Fluent files.
    pub const ALL: [Self; 10] = [
        Self::Heading,
        Self::Line,
        Self::Name,
        Self::Path,
        Self::RuleEmpty,
        Self::RuleDotPart,
        Self::RuleForbiddenCharacter,
        Self::RuleTrailingDotOrSpace,
        Self::RuleDeviceName,
        Self::RuleTooManyBytes,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Heading => "rename-heading",
            Self::Line => "rename-line",
            Self::Name => "rename-name",
            Self::Path => "rename-path",
            Self::RuleEmpty => "rename-rule-empty",
            Self::RuleDotPart => "rename-rule-dot-part",
            Self::RuleForbiddenCharacter => "rename-rule-forbidden-character",
            Self::RuleTrailingDotOrSpace => "rename-rule-trailing-dot-or-space",
            Self::RuleDeviceName => "rename-rule-device-name",
            Self::RuleTooManyBytes => "rename-rule-too-many-bytes",
        }
    }
}

impl From<RenameText> for super::Text {
    fn from(text: RenameText) -> Self {
        Self::Rename(text)
    }
}

/// Returns why the file of a Record differs from its name or record path, in the current language
/// of `texts`; a cause or rule this version does not know yet reads as its English text.
#[must_use]
pub(crate) fn describe_rename(texts: &Texts, cause: RenameCause) -> String {
    let (text, rule) = match cause {
        RenameCause::Name(rule) => (RenameText::Name, rule),
        RenameCause::Path(rule) => (RenameText::Path, rule),
        _ => return cause.to_string(),
    };
    texts.format(text, &[("rule", describe_rule(texts, rule).into())])
}

fn describe_rule(texts: &Texts, rule: NameRule) -> String {
    match rule {
        NameRule::Empty => texts.get(RenameText::RuleEmpty),
        NameRule::DotPart => texts.get(RenameText::RuleDotPart),
        NameRule::ForbiddenCharacter(found) => texts.format(
            RenameText::RuleForbiddenCharacter,
            &[("char", format!("{found:?}").into())],
        ),
        NameRule::TrailingDotOrSpace => texts.get(RenameText::RuleTrailingDotOrSpace),
        NameRule::DeviceName => texts.get(RenameText::RuleDeviceName),
        NameRule::TooManyBytes(bytes) => texts.format(
            RenameText::RuleTooManyBytes,
            &[
                ("bytes", u64::try_from(bytes).unwrap_or(u64::MAX).into()),
                (
                    "max",
                    u64::try_from(MAX_PART_BYTES).unwrap_or(u64::MAX).into(),
                ),
            ],
        ),
        _ => rule.to_string(),
    }
}
