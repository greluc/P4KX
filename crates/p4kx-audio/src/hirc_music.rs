// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The music objects of the `HIRC` section, decoded in bank version 154.

use crate::hirc::SoundSource;

/// The tempo and time signature of a music node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MusicMeter {
    /// The distance between two grid lines, in milliseconds.
    pub grid_period: f64,
    /// The offset of the first grid line, in milliseconds.
    pub grid_offset: f64,
    /// The tempo, in beats per minute.
    pub tempo: f32,
    /// The beats per bar.
    pub beats_per_bar: u8,
    /// The note value of one beat.
    pub beat_value: u8,
}

/// A segment a music node plays when a trigger fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MusicStinger {
    /// The ID of the trigger.
    pub trigger_id: u32,
    /// The ID of the segment it plays.
    pub segment_id: u32,
}

/// The fields every music node shares: its place in the hierarchy, its children, its meter and
/// its stingers.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MusicNode {
    /// The ID of the parent node in the hierarchy.
    pub parent_id: u32,
    /// The IDs of the child nodes, in bank order.
    pub children: Vec<u32>,
    /// The tempo and time signature.
    pub meter: MusicMeter,
    /// The stingers of the node.
    pub stingers: Vec<MusicStinger>,
}

/// A named position in a music segment.
#[derive(Debug, Clone, PartialEq)]
pub struct MusicMarker {
    /// The marker ID.
    pub id: u32,
    /// The position in milliseconds.
    pub position: f64,
    /// The marker name, with invalid UTF-8 replaced.
    pub name: Box<str>,
}

/// A music segment: a stretch of music made of tracks.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MusicSegment {
    /// The shared music node fields; the children are the tracks.
    pub node: MusicNode,
    /// The duration in milliseconds.
    pub duration: f64,
    /// The markers, in bank order.
    pub markers: Vec<MusicMarker>,
}

/// One clip of a music track: a stretch of one source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MusicClip {
    /// The ID of the source the clip plays.
    pub source_id: u32,
    /// The ID of the event the clip posts, 0 for none.
    pub event_id: u32,
    /// Where the clip starts in the track, in milliseconds.
    pub play_at: f64,
    /// How much is cut from the start of the source, in milliseconds.
    pub begin_trim: f64,
    /// How much is cut from the end of the source, in milliseconds.
    pub end_trim: f64,
    /// The duration of the source, in milliseconds.
    pub source_duration: f64,
}

/// How a music track chooses what it plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MusicTrackKind {
    /// Plays its clips, track type 0.
    Normal,
    /// Plays one clip at random, track type 1.
    Random,
    /// Plays its clips in turn, track type 2.
    Sequence,
    /// Plays the clips of the current switch, track type 3.
    Switch,
    /// A track type this reader does not know.
    Other(u8),
}

/// The switch group a switch track follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicTrackSwitch {
    /// The group type: 0 for a switch group, 1 for a state group.
    pub group_type: u8,
    /// The ID of the group.
    pub group_id: u32,
    /// The ID of the switch played when none is set.
    pub default_switch: u32,
    /// The IDs of the switches that select a clip, in clip order.
    pub switches: Vec<u32>,
}

/// A music track: the sources of a segment and the clips that play them.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MusicTrack {
    /// The ID of the parent node in the hierarchy.
    pub parent_id: u32,
    /// The media sources, in bank order.
    pub sources: Vec<SoundSource>,
    /// The clips, in bank order.
    pub clips: Vec<MusicClip>,
    /// How the track chooses what it plays.
    pub kind: MusicTrackKind,
    /// The switch group of a switch track.
    pub switch: Option<MusicTrackSwitch>,
}

/// A music switch container: plays the child that matches the current states or switches.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MusicSwitch {
    /// The shared music node fields.
    pub node: MusicNode,
    /// The number of transition rules.
    pub rule_count: u32,
    /// The IDs of the state or switch groups the decision tree tests, outermost first.
    pub group_ids: Vec<u32>,
    /// The IDs of the nodes the leaves of the decision tree play, in tree order, 0 left out.
    pub targets: Vec<u32>,
}

/// One item of a music playlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicPlaylistItem {
    /// The ID of the segment, 0 for a group of items.
    pub segment_id: u32,
    /// The ID of the item.
    pub item_id: u32,
    /// How many items follow as its children.
    pub children: u32,
    /// How the item chooses among its children: 0 random, 1 sequence continuous, 2 random step,
    /// 3 sequence step.
    pub play_mode: u32,
    /// How often the item repeats, 0 for endlessly.
    pub loops: i16,
    /// The least random repeat count.
    pub loop_min: i16,
    /// The most random repeat count.
    pub loop_max: i16,
    /// The weight among its siblings.
    pub weight: u32,
    /// How many plays pass before the item may repeat.
    pub avoid_repeat_count: u16,
    /// Whether the weight is used.
    pub uses_weight: bool,
    /// Whether the children play in shuffled order.
    pub shuffle: bool,
}

/// A music playlist container.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MusicPlaylist {
    /// The shared music node fields; the children are the segments.
    pub node: MusicNode,
    /// The number of transition rules.
    pub rule_count: u32,
    /// The playlist items in bank order, children after their parent.
    pub items: Vec<MusicPlaylistItem>,
}
