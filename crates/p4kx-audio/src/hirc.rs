use std::ops::Range;

use crate::bank_reader::SectionReader;
use crate::{BankError, SectionId};

const OBJECT_HEADER_LEN: usize = 5;
const ID_LEN: usize = 4;
const FIRST_SOUND_CACHE_ID_VERSION: u32 = 150;
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

impl HircKind {
    /// Returns the kind of the type byte `kind`.
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

/// The action of an Action object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionTarget {
    /// The action type, e.g. `0x0403` for Play.
    pub action_type: u16,
    /// The ID of the object the action is aimed at.
    pub target_id: u32,
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
    /// Any other object, or a Sound of an older bank version; its body is kept raw.
    Raw,
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

/// Parses the `HIRC` section at `base` in the bank, a body of `version`.
pub(crate) fn parse(version: u32, body: &[u8], base: usize) -> Result<Vec<HircObject>, BankError> {
    let mut reader = SectionReader::new(SectionId::HIRC, body);
    let count = reader.u32()?;
    let capacity = usize::try_from(count)
        .unwrap_or(usize::MAX)
        .min(reader.remaining() / (OBJECT_HEADER_LEN + ID_LEN));
    let mut objects = Vec::with_capacity(capacity);
    for _ in 0..count {
        let kind = HircKind::from_byte(reader.u8()?);
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
        let value = value(kind, version, &mut fields)?;
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

fn value(
    kind: HircKind,
    version: u32,
    fields: &mut SectionReader<'_>,
) -> Result<HircValue, BankError> {
    Ok(match kind {
        HircKind::Sound if version >= FIRST_SOUND_CACHE_ID_VERSION => {
            let plugin_id = fields.u32()?;
            let storage = MediaStorage::from_byte(fields.u8()?);
            let source_id = fields.u32()?;
            let _cache_id = fields.u32()?;
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
        _ => HircValue::Raw,
    })
}
