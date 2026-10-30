// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::contract::LocalisedText;
use crate::mission_facts::Levels;
use crate::mission_index::{Broker, MissionIndex, Template};
use crate::mission_layers::{Prop, Property, description_key};
use crate::mission_title::{Token, expand_text};
use crate::texts::Texts;

/// Returns the title and the description of an entry with their `~mission(Token)`s resolved
/// against the properties of the layers, then of the template.
pub(crate) fn titles<'f>(
    levels: &Levels<'_, 'f>,
    (template_data, broker): (Option<&Template<'f>>, Option<&Broker<'f>>),
    (index, texts): (&MissionIndex<'f>, &Texts<'f>),
) -> (LocalisedText<'f>, LocalisedText<'f>) {
    let property = |token: &str| -> Option<&Property<'f>> {
        levels
            .iter()
            .flat_map(|level| level.properties.iter())
            .chain(template_data.into_iter().flat_map(|data| &data.properties))
            .find(|property| property.token.eq_ignore_ascii_case(token))
    };
    let resolve = |token: &str, variant: Option<&str>| -> Option<Token<'f>> {
        match &property(token)?.value {
            Prop::Key(key) => Some(Token::Key(key)),
            Prop::Organization(guid) => index.variant(*guid, variant).map(Token::Key),
            Prop::Placeholder(label) => Some(Token::Placeholder(label)),
            Prop::Unknown => None,
        }
    };
    let title_key = levels
        .iter()
        .find_map(|level| level.title)
        .or_else(|| template_data.and_then(|data| data.title))
        .or_else(|| broker.and_then(|broker| broker.title));
    (
        expand_text(title_key, texts, &resolve),
        expand_text(description_key(levels.iter(), broker), texts, &resolve),
    )
}
