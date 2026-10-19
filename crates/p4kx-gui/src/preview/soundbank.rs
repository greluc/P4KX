use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use p4kx_extraction::{AudioError, EmbeddedWem, Soundbank};
use rayon::prelude::*;

use super::SoundInfo;
use super::audio::AudioPreview;
use super::entry_bytes::{Unreadable, read_whole};
use crate::EntryIndex;
use crate::model::describe;

/// Returns whether `path` names a Wwise soundbank the Audio tab lists the sounds of.
#[must_use]
pub fn is_bank_path(path: &str) -> bool {
    let extension = path
        .len()
        .checked_sub(4)
        .and_then(|start| path.get(start..));
    extension.is_some_and(|extension| extension.eq_ignore_ascii_case(".bnk"))
}

/// One WEM embedded in a soundbank, as the list of the Audio tab shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankSound {
    id: u32,
    size: u64,
    kind: Kind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Sound(SoundInfo),
    Prefetch,
    Plugin,
    Unreadable,
}

impl BankSound {
    fn of(wem: &EmbeddedWem<'_>) -> Self {
        let kind = if wem.is_prefetch() {
            Kind::Prefetch
        } else {
            match wem.parse() {
                Ok(parsed) => Kind::Sound(SoundInfo::of(&parsed)),
                Err(AudioError::PluginMedia) => Kind::Plugin,
                Err(_) => Kind::Unreadable,
            }
        };
        Self {
            id: wem.id(),
            size: wem.bytes().len() as u64,
            kind,
        }
    }

    /// Returns the WEM ID.
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// Returns the size of the WEM in the soundbank, in bytes.
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// Returns whether the soundbank holds only the prefetched start of a streamed WEM, which a
    /// loose WEM Entry of the same ID holds whole.
    #[must_use]
    pub const fn is_prefetch(&self) -> bool {
        matches!(self.kind, Kind::Prefetch)
    }

    /// Returns whether the media is Wwise plug-in data, such as a convolution reverb impulse,
    /// which holds no sound to play.
    #[must_use]
    pub const fn is_plugin(&self) -> bool {
        matches!(self.kind, Kind::Plugin)
    }

    /// Returns whether the WEM can be decoded: it is neither a prefetched start nor plug-in media.
    #[must_use]
    pub const fn is_playable(&self) -> bool {
        !matches!(self.kind, Kind::Prefetch | Kind::Plugin)
    }

    /// Returns what the header says about the sound, or `None` for a prefetched start, plug-in
    /// media and media that is no valid WEM.
    #[must_use]
    pub const fn info(&self) -> Option<&SoundInfo> {
        match &self.kind {
            Kind::Sound(info) => Some(info),
            _ => None,
        }
    }
}
/// Where the preview of a soundbank stands.
#[derive(Debug, Clone, Copy)]
pub enum SoundbankStatus<'a> {
    /// The soundbank is being read.
    Opening,
    /// The soundbank is larger than [`PREVIEW_SIZE_LIMIT`](crate::PREVIEW_SIZE_LIMIT) and was
    /// not read.
    TooLarge,
    /// The soundbank could not be read or parsed, for the reason given, or reading stopped
    /// without a result.
    Failed(Option<&'a str>),
    /// The Archive changed on disk, so the soundbank was not read or its sounds were dropped.
    ArchiveChanged,
    /// The soundbank was parsed; these are the WEMs it embeds in ascending ID order.
    Opened(&'a [BankSound]),
}

#[derive(Debug)]
struct Opened {
    bank: Arc<Soundbank<Vec<u8>>>,
    sounds: Vec<BankSound>,
}

#[derive(Debug)]
enum Event {
    TooLarge,
    Failed(Option<String>),
    ArchiveChanged,
    Opened(Opened),
}

#[derive(Debug)]
enum State {
    Opening,
    TooLarge,
    Failed(Option<String>),
    ArchiveChanged,
    Opened(Opened),
}

#[derive(Debug)]
struct Selected {
    id: u32,
    audio: Option<AudioPreview>,
}

/// The preview of a Wwise soundbank, read and parsed on a background thread.
///
/// It lists the WEMs the soundbank embeds, one of which can be selected and played through an
/// [`AudioPreview`]. Dropping it stops reading and playback.
#[derive(Debug)]
pub struct SoundbankPreview {
    dropped: Arc<AtomicBool>,
    events: Receiver<Event>,
    state: State,
    selected: Option<Selected>,
}

impl SoundbankPreview {
    /// Starts reading the soundbank Entry `id` of `index` on a background thread, calling `wake`
    /// from it once the sounds are listed.
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
            .name("soundbank-preview".to_owned())
            .spawn(move || worker.run(id, &wake));
        let state = match spawned {
            Ok(_handle) => State::Opening,
            Err(_ignored) => State::Failed(None),
        };
        Self {
            dropped,
            events,
            state,
            selected: None,
        }
    }

    /// Returns where the preview stands.
    #[must_use]
    pub fn status(&self) -> SoundbankStatus<'_> {
        match &self.state {
            State::Opening => SoundbankStatus::Opening,
            State::TooLarge => SoundbankStatus::TooLarge,
            State::Failed(reason) => SoundbankStatus::Failed(reason.as_deref()),
            State::ArchiveChanged => SoundbankStatus::ArchiveChanged,
            State::Opened(opened) => SoundbankStatus::Opened(&opened.sounds),
        }
    }

    /// Returns the ID of the selected WEM.
    #[must_use]
    pub fn selected(&self) -> Option<u32> {
        self.selected.as_ref().map(|selected| selected.id)
    }

    /// Returns the selected WEM.
    #[must_use]
    pub fn selected_sound(&self) -> Option<&BankSound> {
        let id = self.selected()?;
        self.sound(id)
    }

    /// Returns the audio preview of the selected WEM, or `None` when none is selected or the
    /// soundbank holds only its prefetched start.
    #[must_use]
    pub fn audio(&self) -> Option<&AudioPreview> {
        self.selected.as_ref()?.audio.as_ref()
    }

    /// Returns the audio preview of the selected WEM for controlling its playback.
    pub fn audio_mut(&mut self) -> Option<&mut AudioPreview> {
        self.selected.as_mut()?.audio.as_mut()
    }

    /// Selects the WEM `id`, unless it is selected already or the soundbank embeds none, and
    /// decodes it on a background thread; `wake` is called from there. Selecting another WEM
    /// stops the one playing.
    pub(crate) fn select(&mut self, id: u32, wake: impl Fn() + Send + 'static) {
        if self.selected() == Some(id) {
            return;
        }
        let State::Opened(opened) = &self.state else {
            return;
        };
        let Some(sound) = self.sound(id) else {
            return;
        };
        let audio = sound
            .is_playable()
            .then(|| AudioPreview::start_embedded(&opened.bank, id, wake));
        self.selected = Some(Selected { id, audio });
    }

    /// Takes the result of the background thread and of the audio preview of the selected WEM;
    /// returns whether one arrived.
    pub(crate) fn poll(&mut self) -> bool {
        let sounded = self.audio_mut().is_some_and(AudioPreview::poll);
        let mut changed = false;
        loop {
            match self.events.try_recv() {
                Ok(event) => {
                    self.apply(event);
                    changed = true;
                }
                Err(TryRecvError::Empty) => return changed || sounded,
                Err(TryRecvError::Disconnected) => {
                    if matches!(self.state, State::Opening) {
                        self.state = State::Failed(None);
                        changed = true;
                    }
                    return changed || sounded;
                }
            }
        }
    }

    fn sound(&self, id: u32) -> Option<&BankSound> {
        let State::Opened(opened) = &self.state else {
            return None;
        };
        let at = opened
            .sounds
            .binary_search_by_key(&id, BankSound::id)
            .ok()?;
        opened.sounds.get(at)
    }

    fn apply(&mut self, event: Event) {
        self.state = match event {
            Event::TooLarge => State::TooLarge,
            Event::Failed(reason) => State::Failed(reason),
            Event::ArchiveChanged => State::ArchiveChanged,
            Event::Opened(opened) => State::Opened(opened),
        };
        if !matches!(self.state, State::Opened(_)) {
            self.selected = None;
        }
    }
}

impl Drop for SoundbankPreview {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

struct Worker {
    index: Arc<EntryIndex>,
    events: Sender<Event>,
    dropped: Arc<AtomicBool>,
}

impl Worker {
    fn run(&self, id: u32, wake: &impl Fn()) {
        let event = self.open(id);
        if !self.dropped.load(Ordering::Acquire) && self.events.send(event).is_ok() {
            wake();
        }
    }

    fn open(&self, id: u32) -> Event {
        let bytes = match read_whole(&self.index, id, &self.dropped) {
            Ok(bytes) => bytes,
            Err(Unreadable::TooLarge) => return Event::TooLarge,
            Err(Unreadable::ArchiveChanged) => return Event::ArchiveChanged,
            Err(Unreadable::Failed(reason)) => return Event::Failed(reason),
        };
        let bank = match Soundbank::parse(bytes) {
            Ok(bank) => Arc::new(bank),
            Err(error) => return Event::Failed(Some(describe(&error))),
        };
        let wems: Vec<EmbeddedWem<'_>> = bank.wems().collect();
        let sounds: Vec<BankSound> = wems.par_iter().map(BankSound::of).collect();
        drop(wems);
        Event::Opened(Opened { bank, sounds })
    }
}
