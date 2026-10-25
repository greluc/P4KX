// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::audio;
use crate::audio_excerpt::{self, Excerpted};
use crate::audio_params::{
    EntitySoundsParams, ExcerptParams, MAX_NAME_LEN, SearchTriggersParams, SoundbankParams,
    TriggerParams, WemParams,
};
use crate::audio_shapes::{
    EntitySoundsResult, ExcerptResult, SoundbankWemsResult, TriggerSearchResult,
    TriggerSoundsResult, WemInfoResult,
};
use crate::audio_wem;
use crate::error::capped;
use crate::noted::Noted;
use crate::page::Page;
use crate::params::MAX_PATH_LEN;
use crate::server::Server;

#[tool_router(router = audio_router, vis = "pub(crate)")]
impl Server {
    #[tool(
        description = "Search the audio triggers and Wwise events of the Archive's ATL definitions by name: search is a \
        substring or a glob with * and ?, ignoring case, matched against trigger and event names; without it every trigger \
        is listed. Each result lists the trigger, the Wwise events it posts with the soundbanks that hold them, why it \
        leads to no soundbank if so, and the Entities that name it (entities: how many to list, 0 skips the lookup, which \
        reads the Record of every Entity once). Paged in definition order with a cursor. The first call of a session reads \
        every ATL definition and soundbank of the Archive. The content may be forwarded by the client to third parties; \
        game texts and media must not be redistributed.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<TriggerSearchResult>(),
        annotations(
            title = "Search audio triggers",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn search_audio_triggers(
        &self,
        Parameters(params): Parameters<SearchTriggersParams>,
    ) -> Result<Noted<TriggerSearchResult>, String> {
        let permit = self.model_permit().await?;
        self.run(move |state| {
            let _permit = permit;
            let search = capped(
                "search",
                params.search.as_deref().unwrap_or_default(),
                MAX_PATH_LEN,
            )?;
            let opened = state.archive(params.archive.as_deref())?;
            let listing = ("search_audio_triggers", opened.version(), search);
            let page = Page::new(&listing, params.cursor.as_deref(), params.limit)?;
            audio::search(&opened, &params, page)
        })
        .await
    }

    #[tool(
        description = "Show the audio chain of an Entity: the audio triggers its Record names (by GUID, record path or \
        class name), each with the ATL definition that defines it, the Wwise events it posts, the soundbanks that hold \
        each event, and why it leads to no soundbank if so. Paged with a cursor. resolve_audio_trigger follows a \
        trigger on to its sounds and WEM IDs. The content may be forwarded by the client to third parties; game texts \
        and media must not be redistributed.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<EntitySoundsResult>(),
        annotations(
            title = "Get Entity sounds",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_entity_sounds(
        &self,
        Parameters(params): Parameters<EntitySoundsParams>,
    ) -> Result<Noted<EntitySoundsResult>, String> {
        let permit = self.model_permit().await?;
        self.run(move |state| {
            let _permit = permit;
            let entity = capped("entity", &params.entity, MAX_PATH_LEN)?;
            let opened = state.archive(params.archive.as_deref())?;
            let listing = ("get_entity_sounds", opened.version(), entity);
            let page = Page::new(&listing, params.cursor.as_deref(), params.limit)?;
            audio::entity_sounds(&opened, &params, page)
        })
        .await
    }

    #[tool(
        description = "Resolve an audio trigger to its sounds: follow each Wwise event through its actions and random, \
        sequence, switch and blend containers and list every variant with its soundbank, WEM ID, how the source is \
        stored, where the WEM is (embedded in a soundbank, or a loose WEM Entry for streamed sounds) and the object \
        path from the event. Localised sounds are resolved for lang, with fallback_lang where that has none. Broken \
        links and their reasons come with the first page. Variants are paged with a cursor. Only paths, IDs and \
        metadata are returned, no audio. The content may be forwarded by the client to third parties; game texts \
        and media must not be redistributed.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<TriggerSoundsResult>(),
        annotations(
            title = "Resolve audio trigger",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn resolve_audio_trigger(
        &self,
        Parameters(params): Parameters<TriggerParams>,
    ) -> Result<Noted<TriggerSoundsResult>, String> {
        let permit = self.model_permit().await?;
        self.run(move |state| {
            let _permit = permit;
            capped("trigger", &params.trigger, MAX_NAME_LEN)?;
            capped(
                "lang",
                params.lang.as_deref().unwrap_or_default(),
                MAX_NAME_LEN,
            )?;
            capped(
                "fallback_lang",
                params.fallback_lang.as_deref().unwrap_or_default(),
                MAX_NAME_LEN,
            )?;
            let opened = state.archive(params.archive.as_deref())?;
            let listing = (
                "resolve_audio_trigger",
                opened.version(),
                &params.trigger,
                &params.lang,
                &params.fallback_lang,
            );
            let page = Page::new(&listing, params.cursor.as_deref(), params.limit)?;
            audio::resolve(&opened, &params, page)
        })
        .await
    }

    #[tool(
        description = "List the WEMs a soundbank (.bnk Entry) embeds, page by page: per WEM its ID, size, codec, \
        channels, sample rate, frame count and duration, whether it only is the prefetched start of a streamed sound \
        (whose whole file is a loose WEM Entry named by the ID) and why it cannot be read if so. Only metadata is \
        returned, no audio. The content may be forwarded by the client to third parties; game texts and media must \
        not be redistributed.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<SoundbankWemsResult>(),
        annotations(
            title = "List soundbank WEMs",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_soundbank_wems(
        &self,
        Parameters(params): Parameters<SoundbankParams>,
    ) -> Result<Noted<SoundbankWemsResult>, String> {
        let permit = self.model_permit().await?;
        self.run(move |state| {
            let _permit = permit;
            let soundbank = capped("soundbank", &params.soundbank, MAX_PATH_LEN)?;
            let opened = state.archive(params.archive.as_deref())?;
            let listing = ("list_soundbank_wems", opened.version(), soundbank);
            let page = Page::new(&listing, params.cursor.as_deref(), params.limit)?;
            audio_wem::soundbank_wems(&opened, &params, page)
        })
        .await
    }

    #[tool(
        description = "Describe one WEM, named by wem_id or by the Entry path of a loose WEM: every place that holds it \
        (embedded in a soundbank, or a loose Entry per language) and, read from the first place that holds the whole \
        sound, its codec, channels, sample rate, frame count, duration and whether it loops. Only metadata is returned, \
        no audio. The content may be forwarded by the client to third parties; game texts and media must not be \
        redistributed.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<WemInfoResult>(),
        annotations(
            title = "Get WEM info",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_wem_info(
        &self,
        Parameters(params): Parameters<WemParams>,
    ) -> Result<Noted<WemInfoResult>, String> {
        let permit = self.model_permit().await?;
        self.run(move |state| {
            let _permit = permit;
            let opened = state.archive(params.archive.as_deref())?;
            audio_wem::info(&opened, &params)
        })
        .await
    }

    #[tool(
        description = "Decode a short excerpt of one WEM, named by wem_id or by the Entry path of a loose WEM, and return \
        it as a 16-bit PCM WAV in an audio content block (audio/wav, base64) next to the metadata. Starts at start_seconds \
        (0 by default), lasts duration_seconds (5 by default, at most 10) and is cut where the WAV would exceed 1 MiB; \
        truncated says so. Sounds that decode to more than 64 MiB, plug-in media and the prefetched start of a streamed \
        sound are refused. The excerpt is for listening and analysis only: audio is game media and must not be \
        redistributed. The content may be forwarded by the client to third parties.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<ExcerptResult>(),
        annotations(
            title = "Get audio excerpt",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_audio_excerpt(
        &self,
        Parameters(params): Parameters<ExcerptParams>,
    ) -> Result<Excerpted, String> {
        let permit = self.model_permit().await?;
        let done = self
            .run(move |state| {
                let _permit = permit;
                let opened = state.archive(params.archive.as_deref())?;
                audio_excerpt::excerpt(&opened, &params)
            })
            .await?;
        let (value, wav) = done.value;
        Ok(Excerpted {
            value,
            wav,
            notice: done.notice,
        })
    }
}
