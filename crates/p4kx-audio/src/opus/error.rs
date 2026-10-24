// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use thiserror::Error;

/// A reason an Opus WEM cannot be repackaged as Ogg Opus or decoded to PCM.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum OpusError {
    /// The `fmt` chunk, the `seek` chunk or the embedded `OpusHead` is malformed.
    #[error("invalid Opus header: {0}")]
    InvalidHeader(&'static str),
    /// The channel layout has no Opus channel mapping.
    #[error("{channels} channels with Wwise mapping {mapping} have no Opus channel mapping")]
    UnsupportedLayout {
        /// Channels of the WEM.
        channels: u16,
        /// The Wwise mapping family of the `fmt` chunk.
        mapping: u8,
    },
    /// A packet lies outside the `data` chunk or exceeds the size limit.
    #[error("Opus packet {index} runs past the data chunk")]
    PacketOutOfBounds {
        /// Index of the packet, counted from 0.
        index: u64,
    },
    /// A packet is malformed.
    #[error("invalid Opus packet {index}: {reason}")]
    InvalidPacket {
        /// Index of the packet, counted from 0.
        index: u64,
        /// What is wrong.
        reason: &'static str,
    },
    /// The Ogg stream embedded in the WEM is malformed.
    #[error("invalid embedded Ogg stream: {0}")]
    InvalidOgg(&'static str),
    /// The packets hold no sample beyond the pre-skip.
    #[error("the Opus stream holds no sample after its pre-skip of {pre_skip}")]
    NoSamples {
        /// Samples the decoder discards at the start.
        pre_skip: u16,
    },
    /// The decoder gives fewer frames than the packets count.
    #[error("the Opus decoder gives {decoded} of the {expected} frames the stream counts")]
    MissingSamples {
        /// Frames the packets count after the pre-skip.
        expected: u64,
        /// Frames the decoder gave.
        decoded: u64,
    },
    /// The Opus decoder fails on a packet.
    #[error("the Opus decoder fails on packet {index}: {reason}")]
    Undecodable {
        /// Index of the packet, counted from 0.
        index: u64,
        /// What the decoder reports.
        reason: String,
    },
}
