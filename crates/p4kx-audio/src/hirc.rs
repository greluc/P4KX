use std::fmt;
use std::ops::Range;

use crate::bank_reader::SectionReader;
use crate::hirc_node::{self, Container};
use crate::{BankError, SectionId};

const OBJECT_HEADER_LEN: usize = 5;
const ID_LEN: usize = 4;
const FIRST_SOUND_VERSION: u32 = 150;
const LAST_SOUND_WITHOUT_CACHE_ID_VERSION: u32 = 150;
const CONTAINER_VERSION: u32 = 154;
const LAST_KNOWN_VERSION: u32 = 154;
const LAST_OLD_TYPE_NUMBERING_VERSION: u32 = 126;
const PLUGIN_TYPE_MASK: u32 = 0x0F;
const SOURCE_PLUGIN_TYPE: u32 = 2;
const PLAY_ACTION_GROUP: u16 = 0x04;
const LAST_FIXED_EVENT_COUNT_VERSION: u32 = 122;

/// The type of a `HIRC` object, from its type byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HircKind {
    /// A state, type 1.
    State,
    /// A sound playing one media source, type 2.
    Sound,
    /// An action such as play or stop, type 3.
    Action,
    /// An event running a list of actions, type 4.
    Event,
    /// A random or sequence container, type 5.
    RandomSequenceContainer,
    /// A switch container, type 6.
    SwitchContainer,
    /// An actor-mixer, type 7.
    ActorMixer,
    /// An audio bus, type 8.
    AudioBus,
    /// A blend or layer container, type 9.
    LayerContainer,
    /// A music segment, type 10.
    MusicSegment,
    /// A music track, type 11.
    MusicTrack,
    /// A music switch container, type 12.
    MusicSwitch,
    /// A music playlist container, type 13.
    MusicPlaylist,
    /// An attenuation, type 14.
    Attenuation,
    /// A dialogue event, type 15.
    DialogueEvent,
    /// An effect share set, type 16.
    FxShareSet,
    /// A custom effect, type 17.
    FxCustom,
    /// An auxiliary bus, type 18.
    AuxiliaryBus,
    /// A low-frequency oscillator, type 19.
    Lfo,
    /// An envelope, type 20.
    Envelope,
    /// An audio device, type 21.
    AudioDevice,
    /// A time modulator, type 22.
    TimeModulator,
    /// A type byte this reader does not know.
    Other(u8),
}

impl fmt::Display for HircKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RandomSequenceContainer => f.write_str("random or sequence container"),
            Self::SwitchContainer => f.write_str("switch container"),
            Self::LayerContainer => f.write_str("blend container"),
            Self::Other(byte) => write!(f, "{byte:#04x}"),
            other => write!(f, "{other:?}"),
        }
    }
}

impl HircKind {
    /// Returns the kind of the type byte `kind` in a bank of `version`.
    ///
    /// Up to version 126 the types from `0x10` on are numbered differently: `0x10` and `0x11` are
    /// a feedback bus and a feedback node, which this reader reports as [`HircKind::Other`], and
    /// `0x12` to `0x17` are the effect share set, custom effect, auxiliary bus, LFO, envelope and
    /// audio device that later versions number `0x10` to `0x15`.
    #[must_use]
    pub const fn of(version: u32, kind: u8) -> Self {
        if version > LAST_OLD_TYPE_NUMBERING_VERSION {
            return Self::from_byte(kind);
        }
        match kind {
            0x10 | 0x11 => Self::Other(kind),
            0x12..=0x17 => Self::from_byte(kind - 2),
            other => Self::from_byte(other),
        }
    }

    /// Returns the kind of the type byte `kind` as banks of version 128 and later number it.
    #[must_use]
    pub const fn from_byte(kind: u8) -> Self {
        match kind {
            1 => Self::State,
            2 => Self::Sound,
            3 => Self::Action,
            4 => Self::Event,
            5 => Self::RandomSequenceContainer,
            6 => Self::SwitchContainer,
            7 => Self::ActorMixer,
            8 => Self::AudioBus,
            9 => Self::LayerContainer,
            10 => Self::MusicSegment,
            11 => Self::MusicTrack,
            12 => Self::MusicSwitch,
            13 => Self::MusicPlaylist,
            14 => Self::Attenuation,
            15 => Self::DialogueEvent,
            16 => Self::FxShareSet,
            17 => Self::FxCustom,
            18 => Self::AuxiliaryBus,
            19 => Self::Lfo,
            20 => Self::Envelope,
            21 => Self::AudioDevice,
            22 => Self::TimeModulator,
            other => Self::Other(other),
        }
    }
}

/// Where the media of a sound is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaStorage {
    /// Embedded in a soundbank, storage 0.
    Embedded,
    /// Its start embedded in a soundbank, the rest streamed from a loose WEM, storage 1.
    PrefetchStreamed,
    /// Streamed from a loose WEM, storage 2.
    Streamed,
    /// A storage value this reader does not know.
    Other(u8),
}

impl MediaStorage {
    const fn from_byte(storage: u8) -> Self {
        match storage {
            0 => Self::Embedded,
            1 => Self::PrefetchStreamed,
            2 => Self::Streamed,
            other => Self::Other(other),
        }
    }
}

/// The media source of a Sound object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundSource {
    /// The Wwise plug-in ID of the codec, e.g. `0x0004_0001` for Vorbis.
    pub plugin_id: u32,
    /// Where the media is stored.
    pub storage: MediaStorage,
    /// The WEM ID of the media.
    pub source_id: u32,
    /// The size of the media in memory as the bank states it.
    pub in_memory_size: u32,
}

impl SoundSource {
    /// Returns whether the source is a plug-in that generates the sound, e.g. silence or a tone,
    /// so that no WEM holds it.
    #[must_use]
    pub const fn is_source_plugin(&self) -> bool {
        self.plugin_id & PLUGIN_TYPE_MASK == SOURCE_PLUGIN_TYPE
    }
}

/// The action of an Action object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionTarget {
    /// The action type, e.g. `0x0403` for Play.
    pub action_type: u16,
    /// The ID of the object the action is aimed at.
    pub target_id: u32,
}

impl ActionTarget {
    /// Returns whether the action plays its target, action types `0x04xx`.
    #[must_use]
    pub const fn is_play(&self) -> bool {
        self.action_type >> 8 == PLAY_ACTION_GROUP
    }
}

/// What this reader decodes from the body of a `HIRC` object.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HircValue {
    /// A Sound object's media source, from bank version 150 on.
    Sound(SoundSource),
    /// An Action object's type and target.
    Action(ActionTarget),
    /// An Event object's actions, in order.
    Event(Vec<u32>),
    /// A random, sequence, switch or blend container's children, in bank version 154.
    Container(Container),
    /// Any other object, or a Sound or container of another bank version; its body is kept raw.
    Raw,
    /// A Sound, Action, Event or container whose body does not hold its fields; its body is kept
    /// raw.
    Malformed(BankError),
}

/// One object of the `HIRC` section, with its ID and its decoded value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HircObject {
    /// The type of the object.
    pub kind: HircKind,
    /// The object ID.
    pub id: u32,
    /// What this reader decodes from the body.
    pub value: HircValue,
    pub(crate) body: Range<usize>,
}

impl HircObject {
    /// Returns why the body could not be decoded, for a [`HircValue::Malformed`] object.
    #[must_use]
    pub const fn error(&self) -> Option<&BankError> {
        match &self.value {
            HircValue::Malformed(error) => Some(error),
            _ => None,
        }
    }
}

/// Parses the body of the `HIRC` section, found at `base` in a bank of `version`.
pub(crate) fn parse(version: u32, body: &[u8], base: usize) -> Result<Vec<HircObject>, BankError> {
    let mut reader = SectionReader::new(SectionId::HIRC, body);
    let count = reader.u32()?;
    let capacity = usize::try_from(count)
        .unwrap_or(usize::MAX)
        .min(reader.remaining() / (OBJECT_HEADER_LEN + ID_LEN));
    let mut objects = Vec::with_capacity(capacity);
    for _ in 0..count {
        let kind = HircKind::of(version, reader.u8()?);
        let len = usize::try_from(reader.u32()?).unwrap_or(usize::MAX);
        if len < ID_LEN {
            return Err(BankError::invalid(
                SectionId::HIRC,
                "an object is shorter than its ID",
            ));
        }
        let range = reader.skip(len)?;
        let object = body.get(range.clone()).unwrap_or_default();
        let mut fields = SectionReader::new(SectionId::HIRC, object);
        let id = fields.u32()?;
        let value = value(kind, version, &mut fields).unwrap_or_else(HircValue::Malformed);
        objects.push(HircObject {
            kind,
            id,
            value,
            body: base + range.start..base + range.end,
        });
    }
    if reader.remaining() > 0 {
        return Err(BankError::invalid(
            SectionId::HIRC,
            "bytes follow the last object",
        ));
    }
    Ok(objects)
}

/// Returns whether a Sound object or a music track playlist item of a bank of `version` carries
/// a `cacheID` before its in-memory media size.
pub(crate) const fn has_cache_id(version: u32) -> bool {
    version > LAST_SOUND_WITHOUT_CACHE_ID_VERSION
}

/// Returns whether an object of `kind` is one this reader decodes but whose layout in a bank of
/// `version` is not known, because the version is newer than 154.
const fn has_unknown_layout(kind: HircKind, version: u32) -> bool {
    version > LAST_KNOWN_VERSION
        && matches!(
            kind,
            HircKind::Sound
                | HircKind::Action
                | HircKind::Event
                | HircKind::RandomSequenceContainer
                | HircKind::SwitchContainer
                | HircKind::LayerContainer
        )
}

/// Decodes the fields of an object after its ID.
///
/// The layouts known are those of bank versions up to 154: a Sound from version 150 (plug-in ID,
/// storage byte, source ID, with a `cacheID` after the source ID from version 151, then the
/// in-memory media size), an Action and an Event from version 49 (an Event counts its actions in
/// 4 bytes up to version 122, in a variable-length integer after), a random, sequence, switch or
/// blend container in version 154 only. The body of such an object in a bank above version 154,
/// whose layouts changed, is kept raw with [`BankError::UnsupportedObjectLayout`]; below its first
/// version it is kept raw without a reason. Other objects are never decoded.
fn value(
    kind: HircKind,
    version: u32,
    fields: &mut SectionReader<'_>,
) -> Result<HircValue, BankError> {
    if has_unknown_layout(kind, version) {
        return Err(BankError::UnsupportedObjectLayout { kind, version });
    }
    Ok(match kind {
        HircKind::Sound if version >= FIRST_SOUND_VERSION => {
            let plugin_id = fields.u32()?;
            let storage = MediaStorage::from_byte(fields.u8()?);
            let source_id = fields.u32()?;
            if has_cache_id(version) {
                fields.u32()?;
            }
            let in_memory_size = fields.u32()?;
            HircValue::Sound(SoundSource {
                plugin_id,
                storage,
                source_id,
                in_memory_size,
            })
        }
        HircKind::Action => HircValue::Action(ActionTarget {
            action_type: fields.u16()?,
            target_id: fields.u32()?,
        }),
        HircKind::Event => {
            let count = if version > LAST_FIXED_EVENT_COUNT_VERSION {
                fields.variable_length()?
            } else {
                fields.u32()?
            };
            let count = usize::try_from(count).unwrap_or(usize::MAX);
            if count > fields.remaining() / ID_LEN {
                return Err(BankError::invalid(
                    SectionId::HIRC,
                    "an event lists more actions than its body holds",
                ));
            }
            let mut actions = Vec::with_capacity(count);
            for _ in 0..count {
                actions.push(fields.u32()?);
            }
            HircValue::Event(actions)
        }
        HircKind::RandomSequenceContainer
        | HircKind::SwitchContainer
        | HircKind::LayerContainer
            if version == CONTAINER_VERSION =>
        {
            HircValue::Container(hirc_node::container(kind, fields)?)
        }
        _ => HircValue::Raw,
    })
}
