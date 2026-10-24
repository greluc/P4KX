// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use super::Pcm;

const NO_SEEK: u64 = u64::MAX;
const SAMPLE_SCALE: f32 = 32_768.0;

/// The playback state the window and the audio thread share.
#[derive(Debug)]
pub(crate) struct Transport {
    playing: AtomicBool,
    stopped: AtomicBool,
    position: AtomicU64,
    seek: AtomicU64,
    volume: AtomicU32,
    stream: AtomicU64,
    failure: Mutex<Option<String>>,
}

impl Transport {
    pub(crate) const fn new(volume: f32) -> Self {
        Self {
            playing: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            position: AtomicU64::new(0),
            seek: AtomicU64::new(NO_SEEK),
            volume: AtomicU32::new(volume.to_bits()),
            stream: AtomicU64::new(0),
            failure: Mutex::new(None),
        }
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Acquire)
    }

    pub(crate) fn set_playing(&self, playing: bool) {
        self.playing.store(playing, Ordering::Release);
    }

    pub(crate) fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
    }

    pub(crate) fn seek(&self, frame: u64) {
        self.seek.store(frame, Ordering::Release);
    }

    /// Returns the frame a pending seek goes to, otherwise the frame the audio thread reached.
    pub(crate) fn position(&self) -> u64 {
        match self.seek.load(Ordering::Acquire) {
            NO_SEEK => self.position.load(Ordering::Acquire),
            frame => frame,
        }
    }

    pub(crate) fn set_volume(&self, volume: f32) {
        self.volume.store(volume.to_bits(), Ordering::Release);
    }

    /// Starts counting a new output stream; reports of earlier streams are ignored from now on.
    pub(crate) fn next_stream(&self) -> u64 {
        self.stream.fetch_add(1, Ordering::AcqRel) + 1
    }

    fn report(&self, stream: u64, reason: String) {
        let mut failure = self.failure.lock().unwrap_or_else(PoisonError::into_inner);
        if self.stream.load(Ordering::Acquire) != stream {
            return;
        }
        if failure.is_none() {
            *failure = Some(reason);
        }
        drop(failure);
        self.set_playing(false);
    }

    pub(crate) fn take_failure(&self) -> Option<String> {
        self.failure
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

/// Reports why an audio output failed to the window, from any thread.
#[derive(Debug, Clone)]
pub struct FailureReporter {
    transport: Arc<Transport>,
    stream: u64,
}

impl FailureReporter {
    /// Stops the playback and shows `reason`; only the first reason of a stream is kept, and
    /// none once the window started another stream.
    pub fn report(&self, reason: impl Into<String>) {
        self.transport.report(self.stream, reason.into());
    }
}

/// The decoded sound an [`AudioOutput`](super::AudioOutput) pulls its samples from, with the
/// play, pause, seek and volume controls of the window applied.
#[derive(Debug)]
pub struct SoundSource {
    pcm: Arc<Pcm>,
    transport: Arc<Transport>,
    stream: u64,
    frame: u64,
    channel: u16,
    playing: bool,
    volume: f32,
}

impl SoundSource {
    pub(crate) fn new(pcm: Arc<Pcm>, transport: Arc<Transport>) -> Self {
        Self {
            stream: transport.next_stream(),
            frame: transport.position(),
            pcm,
            transport,
            channel: 0,
            playing: false,
            volume: 1.0,
        }
    }

    /// Returns the number of interleaved channels, at least 1.
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.pcm.channels()
    }

    /// Returns the sample rate in Hz, at least 1.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.pcm.sample_rate()
    }

    /// Returns a handle that reports a failure of the output to the window.
    #[must_use]
    pub fn reporter(&self) -> FailureReporter {
        FailureReporter {
            transport: Arc::clone(&self.transport),
            stream: self.stream,
        }
    }

    /// Returns the next interleaved sample in `-1.0..1.0` with the volume applied, silence while
    /// paused or after the end, and `None` once the window dropped the sound.
    pub fn next_sample(&mut self) -> Option<f32> {
        if self.channel == 0 && !self.sync() {
            return None;
        }
        let sample = if self.playing {
            f32::from(self.pcm.sample(self.frame, self.channel)) / SAMPLE_SCALE * self.volume
        } else {
            0.0
        };
        self.channel += 1;
        if self.channel >= self.pcm.channels() {
            self.channel = 0;
            if self.playing {
                self.frame += 1;
            }
        }
        Some(sample)
    }

    /// Fills `out` with the next samples; returns how many were written, fewer than `out` holds
    /// only once the window dropped the sound.
    pub fn fill(&mut self, out: &mut [f32]) -> usize {
        for (written, slot) in out.iter_mut().enumerate() {
            match self.next_sample() {
                Some(sample) => *slot = sample,
                None => return written,
            }
        }
        out.len()
    }

    fn sync(&mut self) -> bool {
        let transport = &self.transport;
        if transport.stopped.load(Ordering::Acquire) {
            return false;
        }
        let frames = self.pcm.frames();
        let seek = transport.seek.load(Ordering::Acquire);
        if seek != NO_SEEK {
            self.frame = seek.min(frames);
            transport.position.store(self.frame, Ordering::Release);
            let _taken =
                transport
                    .seek
                    .compare_exchange(seek, NO_SEEK, Ordering::AcqRel, Ordering::Acquire);
        }
        self.playing = transport.is_playing() && self.frame < frames;
        self.volume = f32::from_bits(transport.volume.load(Ordering::Relaxed));
        transport.position.store(self.frame, Ordering::Release);
        true
    }
}
