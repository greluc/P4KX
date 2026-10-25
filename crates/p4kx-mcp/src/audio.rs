// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::sync::Arc;

use p4kx_assembly::Entities;
use p4kx_assembly::audio::{AudioIndex, LanguageChoice, TriggerUsers};
use p4kx_extraction::{DataForge, Record};

use crate::archives::Opened;
use crate::audio_convert as convert;
use crate::audio_params::{
    DEFAULT_ENTITIES, EntitySoundsParams, MAX_ENTITIES, SearchTriggersParams, TriggerParams,
};
use crate::audio_shapes::{
    EntitiesShape, EntityRef, EntityShape, EntitySoundsResult, EntityTriggerShape, LanguageShape,
    TriggerHitShape, TriggerSearchResult, TriggerSoundsResult,
};
use crate::budget::fill;
use crate::error::ToolError;
use crate::page::Page;

const CLASS_PREFIX: &str = "EntityClassDefinition.";
const MAX_BREAKS: usize = 100;

/// Returns one page of the audio triggers whose names or whose events' names match the search.
pub(crate) fn search(
    opened: &Arc<Opened>,
    params: &SearchTriggersParams,
    page: Page,
) -> Result<TriggerSearchResult, ToolError> {
    let index = opened.audio_index();
    let text = params.search.as_deref().unwrap_or_default();
    let found = index.search(text, page.offset(), page.limit());
    let names: Vec<String> = found
        .hits()
        .iter()
        .map(|hit| hit.trigger().name().to_owned())
        .collect();
    let wanted = params
        .entities
        .unwrap_or(DEFAULT_ENTITIES)
        .min(MAX_ENTITIES);
    let (users, entities_skipped, unreadable_records) = if wanted == 0 || names.is_empty() {
        (None, None, 0)
    } else {
        match entities_of(opened, names, usize::try_from(wanted).unwrap_or(usize::MAX)) {
            Ok((users, unreadable)) => (Some(users), None, unreadable),
            Err(error) => (None, Some(error.message()), 0),
        }
    };
    let mut users = users.map(Vec::into_iter);
    let items = found.hits().iter().map(|hit| {
        Ok(TriggerHitShape {
            name: hit.trigger().name().to_owned(),
            definition: hit.definition_path().map(str::to_owned),
            matched_trigger: hit.matched_by_trigger(),
            events: hit
                .events()
                .map(|(event, matched)| convert::event(&event, Some(matched)))
                .collect(),
            unmatched: hit.unmatched().map(convert::unmatched),
            entities: users.as_mut().and_then(Iterator::next),
        })
    });
    let (triggers, next_cursor) = fill(page, found.total(), items, |hit: &TriggerHitShape| {
        format!("audio trigger {}", hit.name)
    })?;
    let (issues, issues_total) = convert::issues(index);
    Ok(TriggerSearchResult {
        archive: opened.archive().path().display().to_string(),
        search: params.search.clone(),
        total: found.total(),
        trigger_count: index.triggers().len(),
        triggers,
        entities_skipped,
        unreadable_records,
        issues,
        issues_total,
        next_cursor,
    })
}

fn entities_of(
    opened: &Arc<Opened>,
    names: Vec<String>,
    listed: usize,
) -> Result<(Vec<EntitiesShape>, usize), ToolError> {
    opened.with_dataforge(move |forge, _| {
        let wanted: HashSet<String> = names.iter().map(|name| name.to_ascii_lowercase()).collect();
        let users = TriggerUsers::build(forge, &move |name| wanted.contains(name));
        let shapes = names
            .iter()
            .map(|name| EntitiesShape {
                count: users.count(name),
                listed: users
                    .entities(forge, name)
                    .take(listed)
                    .map(|record| entity_ref(&record))
                    .collect(),
            })
            .collect();
        Ok((shapes, users.unreadable_records()))
    })
}

fn entity_ref(record: &Record<'_>) -> EntityRef {
    let name = record.name().unwrap_or_default();
    EntityRef {
        guid: record.guid().to_string(),
        record_path: record.record_path().unwrap_or_default().to_owned(),
        class_name: name.strip_prefix(CLASS_PREFIX).unwrap_or(name).to_owned(),
    }
}

/// Returns one page of the audio triggers the Entity `params` names, each with its events and the
/// soundbanks holding them.
pub(crate) fn entity_sounds(
    opened: &Arc<Opened>,
    params: &EntitySoundsParams,
    page: Page,
) -> Result<EntitySoundsResult, ToolError> {
    opened.audio_index();
    let held = Arc::clone(opened);
    let text = params.entity.clone();
    let (entity, triggers, matched) = opened.with_dataforge(move |forge: &DataForge<'_>, _| {
        let record = Entities::new(forge).find(&text)?;
        let index = held.audio_index();
        let audio = index.entity_audio(&record)?;
        let triggers: Vec<EntityTriggerShape> = audio
            .triggers()
            .iter()
            .map(|trigger| EntityTriggerShape {
                name: trigger.name().to_owned(),
                location: trigger.reference().location().to_owned(),
                references: trigger.reference().count(),
                definition: trigger.definition_path().map(str::to_owned),
                events: trigger
                    .events()
                    .map(|event| convert::event(&event, None))
                    .collect(),
                unmatched: trigger.unmatched().map(convert::unmatched),
            })
            .collect();
        Ok((entity_shape(&record), triggers, audio.matched_count()))
    })?;
    let total = triggers.len();
    let items = triggers.into_iter().skip(page.offset()).map(Ok);
    let (listed, next_cursor) = fill(page, total, items, |trigger: &EntityTriggerShape| {
        format!("audio trigger {}", trigger.name)
    })?;
    let (issues, issues_total) = convert::issues(opened.audio_index());
    Ok(EntitySoundsResult {
        archive: opened.archive().path().display().to_string(),
        entity,
        total,
        matched,
        triggers: listed,
        issues,
        issues_total,
        next_cursor,
    })
}

fn entity_shape(record: &Record<'_>) -> EntityShape {
    let name = record.name().unwrap_or_default();
    EntityShape {
        guid: record.guid().to_string(),
        record_path: record.record_path().unwrap_or_default().to_owned(),
        name: name.to_owned(),
        class_name: name.strip_prefix(CLASS_PREFIX).unwrap_or(name).to_owned(),
    }
}

/// Returns the language choice the call names; the languages must be language folders of the
/// Archive, and the fallback defaults to the game's default language.
fn language_choice(
    index: &AudioIndex,
    lang: Option<&str>,
    fallback: Option<&str>,
) -> Result<LanguageChoice, ToolError> {
    let known = |name: &str| {
        index
            .language(name)
            .ok_or_else(|| ToolError::UnknownLanguage {
                language: name.to_owned(),
                available: if index.languages().is_empty() {
                    "none".to_owned()
                } else {
                    index.languages().join(", ")
                },
            })
    };
    let preferred = lang.map(known).transpose()?;
    let fallback = match fallback {
        Some(name) => Some(known(name)?),
        None => index.default_language(),
    };
    Ok(LanguageChoice::new(preferred, fallback))
}

/// Returns one page of the sound variants of the audio trigger `params` names; the broken links
/// come with the first page.
pub(crate) fn resolve(
    opened: &Opened,
    params: &TriggerParams,
    page: Page,
) -> Result<TriggerSoundsResult, ToolError> {
    let audio = opened.audio();
    let index = audio.index();
    let choice = language_choice(
        index,
        params.lang.as_deref(),
        params.fallback_lang.as_deref(),
    )?;
    let sounds = audio
        .resolve_trigger_in(&params.trigger, &choice)
        .ok_or_else(|| ToolError::UnknownTrigger(params.trigger.clone()))?;
    let total = sounds.variants().len();
    let items = page
        .slice(sounds.variants())
        .iter()
        .map(|variant| Ok(convert::variant(variant)));
    let (variants, next_cursor) = fill(page, total, items, |variant| {
        format!("variant of the sound {}", variant.sound_id)
    })?;
    let breaks = if page.offset() == 0 {
        sounds
            .breaks()
            .iter()
            .take(MAX_BREAKS)
            .map(convert::broken_link)
            .collect()
    } else {
        Vec::new()
    };
    let (issues, issues_total) = convert::issues(index);
    Ok(TriggerSoundsResult {
        archive: opened.archive().path().display().to_string(),
        language: LanguageShape {
            requested: choice.preferred().map(str::to_owned),
            fallback: choice.fallback().map(str::to_owned),
        },
        trigger: convert::trigger(&sounds),
        counts: convert::counts(&sounds),
        variants,
        breaks,
        issues,
        issues_total,
        next_cursor,
    })
}
