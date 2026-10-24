// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use egui::{Color32, ColorImage, Context, TextureHandle, TextureId, TextureOptions};
use p4kx_extraction::{RgbaImage, Texture};
use rayon::prelude::*;

use super::TextureInfo;
use crate::EntryIndex;
use crate::model::describe;

static SERIAL: AtomicU64 = AtomicU64::new(0);

/// Where the image preview of a texture stands.
#[derive(Debug, Clone, Copy)]
pub enum ImageStatus<'a> {
    /// The texture's header is being read.
    Opening,
    /// The texture could not be opened, for the reason given, or opening stopped without a
    /// result.
    Failed(Option<&'a str>),
    /// The Archive changed on disk, so the texture was not read or its data was dropped.
    ArchiveChanged,
    /// The header was read; the mip stands as given. A format the preview cannot decode has no
    /// mips to show.
    Opened(&'a TextureInfo, MipStatus<'a>),
}

/// Where the shown mip of an opened texture stands.
#[derive(Debug, Clone, Copy)]
pub enum MipStatus<'a> {
    /// No mip was chosen yet.
    Unchosen,
    /// The mip of this level is being decoded.
    Decoding(u32),
    /// The mip of this level could not be decoded, for the reason given, or decoding stopped
    /// without a result.
    Failed(u32, Option<&'a str>),
    /// The mip of this level was decoded; its texture once the window took it over.
    Decoded(u32, Option<TextureId>),
}

#[derive(Debug)]
enum Event {
    Opened(TextureInfo),
    Failed(Option<String>),
    ArchiveChanged,
    Decoded(u32, Result<ColorImage, String>),
}

enum Mip {
    Unchosen,
    Decoding(u32),
    Failed(u32, Option<String>),
    Decoded {
        level: u32,
        image: Option<ColorImage>,
        texture: Option<TextureHandle>,
    },
}

impl fmt::Debug for Mip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unchosen => f.write_str("Unchosen"),
            Self::Decoding(level) => f.debug_tuple("Decoding").field(level).finish(),
            Self::Failed(level, reason) => {
                f.debug_tuple("Failed").field(level).field(reason).finish()
            }
            Self::Decoded { level, texture, .. } => f
                .debug_struct("Decoded")
                .field("level", level)
                .field("uploaded", &texture.is_some())
                .finish_non_exhaustive(),
        }
    }
}

#[derive(Debug)]
enum State {
    Opening,
    Failed(Option<String>),
    ArchiveChanged,
    Opened(TextureInfo, Mip),
}

/// The image preview of a texture: its header read and its mips decoded on a background
/// thread, one requested level at a time.
#[derive(Debug)]
pub struct ImagePreview {
    serial: u64,
    dropped: Arc<AtomicBool>,
    requests: Sender<u32>,
    events: Receiver<Event>,
    state: State,
}

impl ImagePreview {
    /// Starts opening the texture whose header is the Entry `id` of `index` on a background
    /// thread, calling `wake` from it whenever a result is ready.
    pub(crate) fn start(
        index: &Arc<EntryIndex>,
        id: u32,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let (requests, requested) = mpsc::channel();
        let (sender, events) = mpsc::channel();
        let dropped = Arc::new(AtomicBool::new(false));
        let thread_index = Arc::clone(index);
        let thread_dropped = Arc::clone(&dropped);
        let spawned = thread::Builder::new()
            .name("image-preview".to_owned())
            .spawn(move || {
                let worker = Worker {
                    index: &thread_index,
                    requests: &requested,
                    events: &sender,
                    dropped: &thread_dropped,
                };
                worker.run(id, &wake);
            });
        let state = match spawned {
            Ok(_handle) => State::Opening,
            Err(_ignored) => State::Failed(None),
        };
        Self {
            serial: SERIAL.fetch_add(1, Ordering::Relaxed),
            dropped,
            requests,
            events,
            state,
        }
    }

    /// Returns a number no other image preview of this process has.
    #[must_use]
    pub const fn serial(&self) -> u64 {
        self.serial
    }

    /// Returns where the preview stands.
    #[must_use]
    pub fn status(&self) -> ImageStatus<'_> {
        match &self.state {
            State::Opening => ImageStatus::Opening,
            State::Failed(reason) => ImageStatus::Failed(reason.as_deref()),
            State::ArchiveChanged => ImageStatus::ArchiveChanged,
            State::Opened(info, mip) => ImageStatus::Opened(info, mip_status(mip)),
        }
    }

    /// Returns whether the header is being read or a mip is being decoded.
    #[must_use]
    pub const fn is_busy(&self) -> bool {
        matches!(
            self.state,
            State::Opening | State::Opened(_, Mip::Decoding(_))
        )
    }

    /// Returns the level shown or being decoded, or `None` when none was chosen.
    #[must_use]
    pub const fn level(&self) -> Option<u32> {
        match &self.state {
            State::Opened(
                _,
                Mip::Decoding(level) | Mip::Failed(level, _) | Mip::Decoded { level, .. },
            ) => Some(*level),
            _ => None,
        }
    }

    /// Decodes mip `level` on the background thread unless it is shown or being decoded
    /// already; does nothing before the header was read, for a format the preview cannot
    /// decode, and for a level that is not showable within `max_side` pixels a side.
    pub fn request(&mut self, level: u32, max_side: u32) {
        let State::Opened(info, mip) = &mut self.state else {
            return;
        };
        let current = matches!(
            mip,
            Mip::Decoding(shown) | Mip::Decoded { level: shown, .. } if *shown == level
        );
        if current || !info.is_showable(level, max_side) || !info.is_decodable() {
            return;
        }
        *mip = match self.requests.send(level) {
            Ok(()) => Mip::Decoding(level),
            Err(_closed) => Mip::Failed(level, None),
        };
    }

    /// Hands a decoded mip to `ctx` as a texture once, giving up the decoded pixels.
    pub(crate) fn upload(&mut self, ctx: &Context) {
        if let State::Opened(
            _,
            Mip::Decoded {
                level,
                image,
                texture,
            },
        ) = &mut self.state
            && let Some(image) = image.take()
        {
            *texture =
                Some(ctx.load_texture(format!("p4kx-mip-{level}"), image, TextureOptions::NEAREST));
        }
    }

    /// Takes the results the background thread sent; returns whether one arrived.
    pub(crate) fn poll(&mut self) -> bool {
        let mut changed = false;
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
        match event {
            Event::Opened(info) => self.state = State::Opened(info, Mip::Unchosen),
            Event::Failed(reason) => self.state = State::Failed(reason),
            Event::ArchiveChanged => self.state = State::ArchiveChanged,
            Event::Decoded(level, decoded) => {
                if let State::Opened(_, mip) = &mut self.state
                    && matches!(mip, Mip::Decoding(pending) if *pending == level)
                {
                    *mip = match decoded {
                        Ok(image) => Mip::Decoded {
                            level,
                            image: Some(image),
                            texture: None,
                        },
                        Err(reason) => Mip::Failed(level, Some(reason)),
                    };
                }
            }
        }
    }

    fn lost(&mut self) -> bool {
        match &mut self.state {
            State::Opening => self.state = State::Failed(None),
            State::Opened(_, mip @ Mip::Decoding(_)) => {
                if let Mip::Decoding(level) = *mip {
                    *mip = Mip::Failed(level, None);
                }
            }
            _ => return false,
        }
        true
    }
}

impl Drop for ImagePreview {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

fn mip_status(mip: &Mip) -> MipStatus<'_> {
    match mip {
        Mip::Unchosen => MipStatus::Unchosen,
        Mip::Decoding(level) => MipStatus::Decoding(*level),
        Mip::Failed(level, reason) => MipStatus::Failed(*level, reason.as_deref()),
        Mip::Decoded { level, texture, .. } => {
            MipStatus::Decoded(*level, texture.as_ref().map(TextureHandle::id))
        }
    }
}

/// What the background thread of one image preview works with.
struct Worker<'a> {
    index: &'a EntryIndex,
    requests: &'a Receiver<u32>,
    events: &'a Sender<Event>,
    dropped: &'a AtomicBool,
}

impl Worker<'_> {
    fn send(&self, event: Event, wake: &impl Fn()) -> bool {
        let sent = !self.dropped.load(Ordering::Acquire) && self.events.send(event).is_ok();
        if sent {
            wake();
        }
        sent
    }

    fn is_dropped(&self) -> bool {
        self.dropped.load(Ordering::Acquire)
    }

    fn latest(&self, level: u32) -> u32 {
        self.requests.try_iter().last().unwrap_or(level)
    }

    fn run(&self, id: u32, wake: &impl Fn()) {
        let Some(entry) = self.index.entry(id) else {
            self.send(Event::Failed(None), wake);
            return;
        };
        if self.index.watch().check().is_err() {
            self.send(Event::ArchiveChanged, wake);
            return;
        }
        let texture = match Texture::from_archive(self.index.archive(), entry.path()) {
            Ok(texture) => texture,
            Err(error) => {
                self.send(Event::Failed(Some(describe(&error))), wake);
                return;
            }
        };
        let info = TextureInfo::of(texture.header());
        let decodable = info.is_decodable();
        if !self.send(Event::Opened(info), wake) || !decodable {
            return;
        }
        let mut next = self.requests.recv().ok();
        while let Some(requested) = next {
            let level = self.latest(requested);
            if self.is_dropped() {
                return;
            }
            let decoded = texture.decode_preview(level);
            if self.is_dropped() {
                return;
            }
            if let Some(newer) = self.requests.try_iter().last() {
                next = Some(newer);
                continue;
            }
            let decoded = decoded
                .map(|image| colour_image(&image))
                .map_err(|error| describe(&error));
            if self.index.watch().check().is_err() {
                self.send(Event::ArchiveChanged, wake);
                return;
            }
            if !self.send(Event::Decoded(level, decoded), wake) {
                return;
            }
            next = self.requests.recv().ok();
        }
    }
}
fn colour_image(image: &RgbaImage) -> ColorImage {
    let pixels = image
        .pixels()
        .par_chunks_exact(4)
        .map(|pixel| match *pixel {
            [r, g, b, a] => Color32::from_rgba_unmultiplied(r, g, b, a),
            _ => Color32::TRANSPARENT,
        })
        .collect();
    ColorImage::new([image.width() as usize, image.height() as usize], pixels)
}
