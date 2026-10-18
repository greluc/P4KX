use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use p4kx_extraction::{Codec, Wem};

use super::PREVIEW_SIZE_LIMIT;
use crate::EntryIndex;
use crate::audio::{self, Pcm, Player, Undecoded};
use crate::model::describe;

static SERIAL: AtomicU64 = AtomicU64::new(0);

/// Returns whether `path` names a Wwise WEM sound the Audio tab plays.
#[must_use]
pub fn is_sound_path(path: &str) -> bool {
    let extension = path
        .len()
        .checked_sub(4)
        .and_then(|start| path.get(start..));
    extension.is_some_and(|extension| extension.eq_ignore_ascii_case(".wem"))
}

/// What the header of a WEM says about the sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundInfo {
    /// The codec.
    pub codec: Codec,
    /// The number of channels.
    pub channels: u16,
    /// The sample rate in Hz.
    pub sample_rate: u32,
    /// The playing time, when the header states it or the codec allows computing it.
    pub duration: Option<Duration>,
}

/// Where the audio preview of a WEM stands.
#[derive(Debug, Clone, Copy)]
pub enum AudioStatus<'a> {
    /// The WEM is being read.
    Opening,
    /// The WEM is larger than [`PREVIEW_SIZE_LIMIT`] and was not read.
    TooLarge,
    /// The WEM could not be read, for the reason given, or reading stopped without a result.
    Failed(Option<&'a str>),
    /// The Archive changed on disk, so the WEM was not read or its sound was dropped.
    ArchiveChanged,
    /// The header was read; the sound stands as given.
    Opened(&'a SoundInfo, SoundStatus<'a>),
}

/// Where the decoded sound of an opened WEM stands.
#[derive(Debug, Clone, Copy)]
pub enum SoundStatus<'a> {
    /// The sound is being decoded.
    Decoding,
    /// The preview has no decoder for the codec.
    Unsupported(Codec),
    /// The decoded sound would exceed [`MAX_PREVIEW_PCM`](crate::MAX_PREVIEW_PCM).
    TooLong,
    /// The sound could not be decoded, for the reason given, or decoding stopped without a
    /// result.
    Failed(Option<&'a str>),
    /// The sound was decoded and plays through this player.
    Ready(&'a Player),
}

#[derive(Debug)]
enum Event {
    TooLarge,
    Failed(Option<String>),
    ArchiveChanged,
    Opened(SoundInfo),
    Decoded(Result<Pcm, Undecoded>),
}

#[derive(Debug)]
enum Sound {
    Decoding,
    Unsupported(Codec),
    TooLong,
    Failed(Option<String>),
    Ready(Player),
}

#[derive(Debug)]
enum State {
    Opening,
    TooLarge,
    Failed(Option<String>),
    ArchiveChanged,
    Opened(SoundInfo, Sound),
}

/// The audio preview of a WEM: read and decoded to PCM on a background thread, then played
/// through a [`Player`]. Dropping it stops decoding and playback.
#[derive(Debug)]
pub struct AudioPreview {
    serial: u64,
    dropped: Arc<AtomicBool>,
    events: Receiver<Event>,
    state: State,
}

impl AudioPreview {
    /// Starts reading and decoding the WEM Entry `id` of `index` on a background thread,
    /// calling `wake` from it whenever a result is ready.
    pub(crate) fn start(
        index: &Arc<EntryIndex>,
        id: u32,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let (sender, events) = mpsc::channel();
        let dropped = Arc::new(AtomicBool::new(false));
        let worker = Worker {
            index: Arc::clone(index),
            events: sender,
            dropped: Arc::clone(&dropped),
        };
        let spawned = thread::Builder::new()
            .name("audio-preview".to_owned())
            .spawn(move || worker.run(id, &wake));
        let state = match spawned {
            Ok(_handle) => State::Opening,
            Err(_ignored) => State::Failed(None),
        };
        Self {
            serial: SERIAL.fetch_add(1, Ordering::Relaxed),
            dropped,
            events,
            state,
        }
    }

    /// Returns a number no other audio preview of this process has.
    #[must_use]
    pub const fn serial(&self) -> u64 {
        self.serial
    }

    /// Returns where the preview stands.
    #[must_use]
    pub fn status(&self) -> AudioStatus<'_> {
        match &self.state {
            State::Opening => AudioStatus::Opening,
            State::TooLarge => AudioStatus::TooLarge,
            State::Failed(reason) => AudioStatus::Failed(reason.as_deref()),
            State::ArchiveChanged => AudioStatus::ArchiveChanged,
            State::Opened(info, sound) => AudioStatus::Opened(info, sound_status(sound)),
        }
    }

    /// Returns whether the WEM is being read or decoded.
    #[must_use]
    pub const fn is_busy(&self) -> bool {
        matches!(
            self.state,
            State::Opening | State::Opened(_, Sound::Decoding)
        )
    }

    /// Returns the player once the sound was decoded.
    #[must_use]
    pub const fn player(&self) -> Option<&Player> {
        match &self.state {
            State::Opened(_, Sound::Ready(player)) => Some(player),
            _ => None,
        }
    }

    /// Returns the player for controlling it, once the sound was decoded.
    pub const fn player_mut(&mut self) -> Option<&mut Player> {
        match &mut self.state {
            State::Opened(_, Sound::Ready(player)) => Some(player),
            _ => None,
        }
    }

    /// Takes the results of the background thread and a failure of the audio output; returns
    /// whether one arrived.
    pub(crate) fn poll(&mut self) -> bool {
        let mut changed = self.player_mut().is_some_and(Player::poll);
        loop {
            match self.events.try_recv() {
                Ok(event) => {
                    self.apply(event);
                    changed = true;
                }
                Err(TryRecvError::Empty) => return changed,
                Err(TryRecvError::Disconnected) => return self.lost() || changed,
            }
        }
    }

    fn apply(&mut self, event: Event) {
        self.state = match event {
            Event::TooLarge => State::TooLarge,
            Event::Failed(reason) => State::Failed(reason),
            Event::ArchiveChanged => State::ArchiveChanged,
            Event::Opened(info) => State::Opened(info, Sound::Decoding),
            Event::Decoded(decoded) => {
                let State::Opened(info, Sound::Decoding) = self.state else {
                    return;
                };
                let sound = match decoded {
                    Ok(pcm) => Sound::Ready(Player::new(pcm, 1.0)),
                    Err(Undecoded::Unsupported(codec)) => Sound::Unsupported(codec),
                    Err(Undecoded::TooLong) => Sound::TooLong,
                    Err(Undecoded::Cancelled) => Sound::Failed(None),
                    Err(Undecoded::Failed(reason)) => Sound::Failed(Some(reason)),
                };
                State::Opened(info, sound)
            }
        };
    }

    fn lost(&mut self) -> bool {
        match &mut self.state {
            State::Opening => self.state = State::Failed(None),
            State::Opened(_, sound @ Sound::Decoding) => *sound = Sound::Failed(None),
            _ => return false,
        }
        true
    }
}

impl Drop for AudioPreview {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

fn sound_status(sound: &Sound) -> SoundStatus<'_> {
    match sound {
        Sound::Decoding => SoundStatus::Decoding,
        Sound::Unsupported(codec) => SoundStatus::Unsupported(*codec),
        Sound::TooLong => SoundStatus::TooLong,
        Sound::Failed(reason) => SoundStatus::Failed(reason.as_deref()),
        Sound::Ready(player) => SoundStatus::Ready(player),
    }
}

/// What the background thread of one audio preview works with.
struct Worker {
    index: Arc<EntryIndex>,
    events: Sender<Event>,
    dropped: Arc<AtomicBool>,
}

impl Worker {
    fn send(&self, event: Event, wake: &impl Fn()) -> bool {
        let sent = !self.dropped.load(Ordering::Acquire) && self.events.send(event).is_ok();
        if sent {
            wake();
        }
        sent
    }

    fn run(&self, id: u32, wake: &impl Fn()) {
        let event = match self.open(id) {
            Ok(wem) => {
                let info = SoundInfo {
                    codec: wem.codec(),
                    channels: wem.channels(),
                    sample_rate: wem.sample_rate(),
                    duration: wem.duration(),
                };
                if !self.send(Event::Opened(info), wake) {
                    return;
                }
                let decoded = audio::decode(&wem, &self.dropped);
                drop(wem);
                if self.index.watch().check().is_err() {
                    Event::ArchiveChanged
                } else {
                    Event::Decoded(decoded)
                }
            }
            Err(event) => event,
        };
        self.send(event, wake);
    }

    fn open(&self, id: u32) -> Result<Wem<Vec<u8>>, Event> {
        let entry = self.index.entry(id).ok_or(Event::Failed(None))?;
        if self.index.watch().check().is_err() {
            return Err(Event::ArchiveChanged);
        }
        if entry.uncompressed_size() > PREVIEW_SIZE_LIMIT {
            return Err(Event::TooLarge);
        }
        let failed = |error: &dyn std::error::Error| Event::Failed(Some(describe(error)));
        let reader = self
            .index
            .archive()
            .open_entry(entry)
            .map_err(|error| failed(&error))?;
        let capacity = usize::try_from(entry.uncompressed_size()).unwrap_or(0);
        let mut bytes = Vec::with_capacity(capacity);
        let reader = Cancellable {
            inner: reader,
            dropped: &self.dropped,
        };
        reader
            .take(PREVIEW_SIZE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| failed(&error))?;
        if bytes.len() as u64 > PREVIEW_SIZE_LIMIT {
            return Err(Event::TooLarge);
        }
        if self.index.watch().check().is_err() {
            return Err(Event::ArchiveChanged);
        }
        Wem::parse(bytes).map_err(|error| failed(&error))
    }
}

/// A reader that fails once the preview it reads for was dropped.
struct Cancellable<'a, R> {
    inner: R,
    dropped: &'a AtomicBool,
}

impl<R: Read> Read for Cancellable<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.dropped.load(Ordering::Acquire) {
            return Err(std::io::Error::other("the preview was closed"));
        }
        self.inner.read(buf)
    }
}
