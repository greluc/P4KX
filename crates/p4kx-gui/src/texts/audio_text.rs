/// A text of the audio preview of a WEM, looked up in the Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioText {
    /// The preview tab that plays a sound.
    Tab,
    /// The key of the codec.
    Codec,
    /// The key of the number of channels.
    Channels,
    /// The key of the sample rate.
    SampleRate,
    /// The key of the playing time.
    Duration,
    /// A sample rate; takes `rate`.
    Hertz,
    /// Shown for a value the WEM does not state, such as an unknown playing time.
    Unknown,
    /// The action that plays the sound.
    Play,
    /// The action that pauses the sound.
    Pause,
    /// The accessible name of the position bar.
    Position,
    /// The label and accessible name of the volume bar.
    Volume,
    /// The position and the playing time; takes `position` and `duration`.
    Time,
    /// Shown while the sound is decoded; takes `path`.
    Decoding,
    /// Shown for a codec the preview cannot decode; takes `codec` and `path`.
    Unsupported,
    /// Shown when the decoded sound would be too large; takes `path` and `limit`.
    TooLong,
    /// Shown when the sound cannot be read or decoded; takes `path` and `reason`.
    Failed,
    /// Shown when the audio output fails; takes `reason`.
    OutputFailed,
}

impl AudioText {
    /// Every text of the audio preview.
    pub(crate) const ALL: [Self; 17] = [
        Self::Tab,
        Self::Codec,
        Self::Channels,
        Self::SampleRate,
        Self::Duration,
        Self::Hertz,
        Self::Unknown,
        Self::Play,
        Self::Pause,
        Self::Position,
        Self::Volume,
        Self::Time,
        Self::Decoding,
        Self::Unsupported,
        Self::TooLong,
        Self::Failed,
        Self::OutputFailed,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Tab => "preview-tab-audio",
            Self::Codec => "audio-codec",
            Self::Channels => "audio-channels",
            Self::SampleRate => "audio-sample-rate",
            Self::Duration => "audio-duration",
            Self::Hertz => "audio-hertz",
            Self::Unknown => "audio-unknown",
            Self::Play => "audio-play",
            Self::Pause => "audio-pause",
            Self::Position => "audio-position",
            Self::Volume => "audio-volume",
            Self::Time => "audio-time",
            Self::Decoding => "audio-decoding",
            Self::Unsupported => "audio-unsupported",
            Self::TooLong => "audio-too-long",
            Self::Failed => "audio-failed",
            Self::OutputFailed => "audio-output-failed",
        }
    }
}
