// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{
    ContainerKind, HircKind, HircLayout, HircObject, HircValue, MediaStorage, MusicClip,
    MusicMarker, MusicPlaylist, MusicSegment, MusicStinger, MusicSwitch, MusicTrack,
    MusicTrackKind, MusicTrackSwitch, SoundSource,
};
use serde::Serialize;

/// One `HIRC` object as `p4kx bnk objects` prints it, in text and as JSON.
#[derive(Serialize)]
pub(crate) struct ObjectJson<'a> {
    pub(crate) index: usize,
    pub(crate) id: u32,
    #[serde(rename = "type")]
    pub(crate) kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) type_byte: Option<u8>,
    pub(crate) status: &'static str,
    pub(crate) error: Option<String>,
    pub(crate) parent: Option<u32>,
    pub(crate) children: Vec<u32>,
    pub(crate) detail: Option<Detail<'a>>,
}

/// The fields of an object that depend on its type.
#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum Detail<'a> {
    Sound(SoundDetail),
    Action(ActionDetail),
    Container(ContainerDetail),
    Segment(SegmentDetail<'a>),
    Track(TrackDetail<'a>),
    Switch(SwitchDetail<'a>),
    Playlist(PlaylistDetail),
}

#[derive(Serialize)]
pub(crate) struct SoundDetail {
    pub(crate) plugin_id: u32,
    pub(crate) storage: &'static str,
    pub(crate) source_id: u32,
    pub(crate) in_memory_size: u32,
    pub(crate) language_specific: bool,
    pub(crate) source_plugin: bool,
}

#[derive(Serialize)]
pub(crate) struct ActionDetail {
    pub(crate) action_type: u16,
    pub(crate) target: u32,
    pub(crate) soundbank_id: Option<u32>,
}

#[derive(Serialize)]
pub(crate) struct ContainerDetail {
    pub(crate) container_kind: &'static str,
}

#[derive(Serialize)]
pub(crate) struct SegmentDetail<'a> {
    pub(crate) duration_ms: f64,
    pub(crate) tempo: f32,
    pub(crate) beats_per_bar: u8,
    pub(crate) beat_value: u8,
    pub(crate) markers: Vec<MarkerJson<'a>>,
    pub(crate) stingers: Vec<StingerJson>,
}

#[derive(Serialize)]
pub(crate) struct MarkerJson<'a> {
    pub(crate) id: u32,
    pub(crate) position_ms: f64,
    pub(crate) name: &'a str,
}

#[derive(Serialize)]
pub(crate) struct StingerJson {
    pub(crate) trigger_id: u32,
    pub(crate) segment_id: u32,
}

#[derive(Serialize)]
pub(crate) struct TrackDetail<'a> {
    pub(crate) track_kind: &'static str,
    pub(crate) clips: Vec<ClipJson>,
    pub(crate) switch: Option<TrackSwitchJson<'a>>,
}

#[derive(Serialize)]
pub(crate) struct ClipJson {
    pub(crate) source_id: u32,
    pub(crate) event_id: u32,
    pub(crate) play_at_ms: f64,
    pub(crate) begin_trim_ms: f64,
    pub(crate) end_trim_ms: f64,
    pub(crate) source_duration_ms: f64,
}

#[derive(Serialize)]
pub(crate) struct TrackSwitchJson<'a> {
    pub(crate) group_type: u8,
    pub(crate) group_id: u32,
    pub(crate) default_switch: u32,
    pub(crate) switches: &'a [u32],
}

#[derive(Serialize)]
pub(crate) struct SwitchDetail<'a> {
    pub(crate) rule_count: u32,
    pub(crate) group_ids: &'a [u32],
    pub(crate) targets: &'a [u32],
}

#[derive(Serialize)]
pub(crate) struct PlaylistDetail {
    pub(crate) rule_count: u32,
    pub(crate) items: Vec<ItemJson>,
}

#[derive(Serialize)]
pub(crate) struct ItemJson {
    pub(crate) depth: usize,
    pub(crate) item_id: u32,
    pub(crate) segment_id: u32,
    pub(crate) children: u32,
    pub(crate) play_mode: u32,
    pub(crate) loops: i16,
    pub(crate) loop_min: i16,
    pub(crate) loop_max: i16,
    pub(crate) weight: u32,
    pub(crate) avoid_repeat_count: u16,
    pub(crate) uses_weight: bool,
    pub(crate) shuffle: bool,
}

impl<'a> ObjectJson<'a> {
    /// Describes the object at `index` in bank order.
    pub(crate) fn new(index: usize, object: &'a HircObject) -> Self {
        Self {
            index,
            id: object.id,
            kind: object.kind.slug(),
            type_byte: match object.kind {
                HircKind::Other(byte) => Some(byte),
                _ => None,
            },
            status: status(object.layout()),
            error: object.error().map(ToString::to_string),
            parent: object.parent_id().filter(|parent| *parent != 0),
            children: object.children().collect(),
            detail: detail(&object.value),
        }
    }
}

const fn status(layout: HircLayout) -> &'static str {
    match layout {
        HircLayout::Decoded => "decoded",
        HircLayout::Undecoded => "undecoded",
        HircLayout::Unsupported => "unsupported",
        HircLayout::Malformed => "malformed",
    }
}

fn detail(value: &HircValue) -> Option<Detail<'_>> {
    Some(match value {
        HircValue::Sound(source) => Detail::Sound(sound(source)),
        HircValue::Action(action) => Detail::Action(ActionDetail {
            action_type: action.action_type,
            target: action.target_id,
            soundbank_id: action.bank.map(|bank| bank.bank_id),
        }),
        HircValue::Container(container) => Detail::Container(ContainerDetail {
            container_kind: container_kind(container.kind),
        }),
        HircValue::MusicSegment(segment) => Detail::Segment(segment_detail(segment)),
        HircValue::MusicTrack(track) => Detail::Track(track_detail(track)),
        HircValue::MusicSwitch(switch) => Detail::Switch(switch_detail(switch)),
        HircValue::MusicPlaylist(playlist) => Detail::Playlist(playlist_detail(playlist)),
        HircValue::Event(_) | HircValue::Raw | HircValue::Malformed(_) => return None,
    })
}

const fn sound(source: &SoundSource) -> SoundDetail {
    SoundDetail {
        plugin_id: source.plugin_id,
        storage: match source.storage {
            MediaStorage::Embedded => "embedded",
            MediaStorage::PrefetchStreamed => "prefetch-streamed",
            MediaStorage::Streamed => "streamed",
            MediaStorage::Other(_) => "other",
        },
        source_id: source.source_id,
        in_memory_size: source.in_memory_size,
        language_specific: source.is_language_specific(),
        source_plugin: source.is_source_plugin(),
    }
}

const fn container_kind(kind: ContainerKind) -> &'static str {
    match kind {
        ContainerKind::Random => "random",
        ContainerKind::Sequence => "sequence",
        ContainerKind::Switch => "switch",
        ContainerKind::Blend => "blend",
        _ => "other",
    }
}

fn segment_detail(segment: &MusicSegment) -> SegmentDetail<'_> {
    let meter = &segment.node.meter;
    SegmentDetail {
        duration_ms: segment.duration,
        tempo: meter.tempo,
        beats_per_bar: meter.beats_per_bar,
        beat_value: meter.beat_value,
        markers: segment.markers.iter().map(marker).collect(),
        stingers: segment.node.stingers.iter().copied().map(stinger).collect(),
    }
}

fn marker(marker: &MusicMarker) -> MarkerJson<'_> {
    MarkerJson {
        id: marker.id,
        position_ms: marker.position,
        name: &marker.name,
    }
}

const fn stinger(stinger: MusicStinger) -> StingerJson {
    StingerJson {
        trigger_id: stinger.trigger_id,
        segment_id: stinger.segment_id,
    }
}

fn track_detail(track: &MusicTrack) -> TrackDetail<'_> {
    TrackDetail {
        track_kind: match track.kind {
            MusicTrackKind::Normal => "normal",
            MusicTrackKind::Random => "random",
            MusicTrackKind::Sequence => "sequence",
            MusicTrackKind::Switch => "switch",
            MusicTrackKind::Other(_) => "other",
        },
        clips: track.clips.iter().map(clip).collect(),
        switch: track.switch.as_ref().map(track_switch),
    }
}

const fn clip(clip: &MusicClip) -> ClipJson {
    ClipJson {
        source_id: clip.source_id,
        event_id: clip.event_id,
        play_at_ms: clip.play_at,
        begin_trim_ms: clip.begin_trim,
        end_trim_ms: clip.end_trim,
        source_duration_ms: clip.source_duration,
    }
}

fn track_switch(switch: &MusicTrackSwitch) -> TrackSwitchJson<'_> {
    TrackSwitchJson {
        group_type: switch.group_type,
        group_id: switch.group_id,
        default_switch: switch.default_switch,
        switches: &switch.switches,
    }
}

fn switch_detail(switch: &MusicSwitch) -> SwitchDetail<'_> {
    SwitchDetail {
        rule_count: switch.rule_count,
        group_ids: &switch.group_ids,
        targets: &switch.targets,
    }
}

fn playlist_detail(playlist: &MusicPlaylist) -> PlaylistDetail {
    PlaylistDetail {
        rule_count: playlist.rule_count,
        items: playlist
            .items
            .iter()
            .zip(playlist.item_depths())
            .map(|(item, depth)| ItemJson {
                depth,
                item_id: item.item_id,
                segment_id: item.segment_id,
                children: item.children,
                play_mode: item.play_mode,
                loops: item.loops,
                loop_min: item.loop_min,
                loop_max: item.loop_max,
                weight: item.weight,
                avoid_repeat_count: item.avoid_repeat_count,
                uses_weight: item.uses_weight,
                shuffle: item.shuffle,
            })
            .collect(),
    }
}
