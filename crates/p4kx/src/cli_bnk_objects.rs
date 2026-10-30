// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use clap::{Args, ValueEnum};
use p4kx_extraction::HircKind;

use crate::cli_bnk::BnkSource;

/// Arguments of `p4kx bnk objects`.
#[derive(Debug, Args)]
pub(crate) struct BnkObjectsArgs {
    /// The soundbank.
    #[command(flatten)]
    pub(crate) source: BnkSource,
    /// Lists only objects of this type.
    #[arg(long = "type", value_enum, value_name = "TYPE")]
    pub(crate) kind: Option<ObjectType>,
    /// Lists only the object with this ID, in decimal or as 0x followed by hexadecimal.
    #[arg(long, value_name = "ID", value_parser = parse_id)]
    pub(crate) id: Option<u32>,
    /// The most objects listed.
    #[arg(
        long,
        default_value_t = 100,
        value_name = "COUNT",
        value_parser = clap::value_parser!(u32).range(1..=10_000)
    )]
    pub(crate) limit: u32,
    /// The number of objects to skip, as the summary line of the previous page gives it.
    #[arg(long, default_value_t = 0, value_name = "COUNT")]
    pub(crate) offset: u32,
}

/// The values of `p4kx bnk objects --type`: the `HIRC` object types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ObjectType {
    State,
    Sound,
    Action,
    Event,
    RandomSequenceContainer,
    SwitchContainer,
    ActorMixer,
    AudioBus,
    LayerContainer,
    MusicSegment,
    MusicTrack,
    MusicSwitch,
    MusicPlaylist,
    Attenuation,
    DialogueEvent,
    FxShareSet,
    FxCustom,
    AuxiliaryBus,
    Lfo,
    Envelope,
    AudioDevice,
    TimeModulator,
}

impl ObjectType {
    /// Returns the object type this value selects.
    pub(crate) const fn kind(self) -> HircKind {
        match self {
            Self::State => HircKind::State,
            Self::Sound => HircKind::Sound,
            Self::Action => HircKind::Action,
            Self::Event => HircKind::Event,
            Self::RandomSequenceContainer => HircKind::RandomSequenceContainer,
            Self::SwitchContainer => HircKind::SwitchContainer,
            Self::ActorMixer => HircKind::ActorMixer,
            Self::AudioBus => HircKind::AudioBus,
            Self::LayerContainer => HircKind::LayerContainer,
            Self::MusicSegment => HircKind::MusicSegment,
            Self::MusicTrack => HircKind::MusicTrack,
            Self::MusicSwitch => HircKind::MusicSwitch,
            Self::MusicPlaylist => HircKind::MusicPlaylist,
            Self::Attenuation => HircKind::Attenuation,
            Self::DialogueEvent => HircKind::DialogueEvent,
            Self::FxShareSet => HircKind::FxShareSet,
            Self::FxCustom => HircKind::FxCustom,
            Self::AuxiliaryBus => HircKind::AuxiliaryBus,
            Self::Lfo => HircKind::Lfo,
            Self::Envelope => HircKind::Envelope,
            Self::AudioDevice => HircKind::AudioDevice,
            Self::TimeModulator => HircKind::TimeModulator,
        }
    }
}

const HEX_PREFIX: &str = "0x";

fn parse_id(text: &str) -> Result<u32, String> {
    let parsed = text
        .strip_prefix(HEX_PREFIX)
        .map_or_else(|| text.parse(), |digits| u32::from_str_radix(digits, 16));
    parsed.map_err(|error| format!("{text:?} is not an object ID: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_type_value_is_the_slug_of_its_kind() {
        for value in ObjectType::value_variants() {
            let name = value
                .to_possible_value()
                .map(|possible| possible.get_name().to_owned());
            assert_eq!(name.as_deref(), Some(value.kind().slug()));
        }
    }

    #[test]
    fn ids_are_decimal_or_hexadecimal() {
        assert_eq!(parse_id("255"), Ok(255));
        assert_eq!(parse_id("0xFF"), Ok(255));
        assert!(parse_id("0x").is_err());
        assert!(parse_id("-1").is_err());
        assert!(parse_id("4294967296").is_err());
    }
}
