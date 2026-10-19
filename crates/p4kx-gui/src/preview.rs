#[cfg(test)]
mod archive_change_tests;
mod contents;
mod image;
mod texture_info;
mod xml;
mod xml_document;

use std::io::Read;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_extraction::{Entry, Texture};

pub use contents::PreviewContents;
pub(crate) use contents::{HEX_ROW_LEN, printable};
pub use image::{ImagePreview, ImageStatus, MipStatus};
pub use texture_info::{MAX_PREVIEW_PIXELS, TextureInfo};
use xml::is_cryxml;
pub use xml::{XML_SIZE_LIMIT, XmlPreview, XmlStatus};
pub use xml_document::{XML_TEXT_LIMIT, XmlDocument};

use crate::EntryIndex;
use crate::model::describe;

/// The largest Entry, in bytes, the window previews; larger Entries are not read.
pub const PREVIEW_SIZE_LIMIT: u64 = 256 * 1024 * 1024;

/// The most bytes a preview reads from the start of an Entry; the rest is left out.
pub const PREVIEW_READ_LIMIT: usize = 1024 * 1024;

/// The tab a preview is shown in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PreviewTab {
    /// The data decoded as text, line by line.
    #[default]
    Text,
    /// The data as offsets, hexadecimal bytes and printable characters.
    Hex,
    /// A `CryXml` Entry converted to XML.
    Xml,
    /// A texture decoded as an image, one mip at a time.
    Image,
    /// The summary of the Archive's DataForge instead of the selected Entry.
    DataForge,
}

/// Where the preview of an Entry stands.
#[derive(Debug, Clone, Copy)]
pub enum PreviewStatus<'a> {
    /// The Entry is being read.
    Loading,
    /// The Entry is larger than [`PREVIEW_SIZE_LIMIT`] and was not read.
    TooLarge,
    /// The Entry could not be read, for the reason given, or reading stopped without a result.
    Failed(Option<&'a str>),
    /// The Archive changed on disk, so the Entry was not read or its data was dropped.
    ArchiveChanged,
    /// The Entry was read.
    Ready(&'a PreviewContents),
}

#[derive(Debug)]
enum Unread {
    Failed(Option<String>),
    ArchiveChanged,
}

#[derive(Debug)]
struct Loaded {
    contents: PreviewContents,
    xml: Option<XmlPreview>,
}

type ReadResult = Result<Loaded, Unread>;

#[derive(Debug)]
enum State {
    Loading(Receiver<ReadResult>),
    TooLarge,
    Failed(Option<String>),
    ArchiveChanged,
    Ready(PreviewContents),
}

/// The preview of one Entry, read on a background thread.
#[derive(Debug)]
pub struct Preview {
    entry: u32,
    state: State,
    image: Option<ImagePreview>,
    xml: Option<XmlPreview>,
    xml_arrived: bool,
}

impl Preview {
    /// Starts reading the Entry `id` of `index` on a background thread, calling `wake` from it
    /// once the data is ready; reads on the calling thread when no thread can be started. A
    /// texture's header also starts its [`ImagePreview`] on a thread of its own.
    ///
    /// Nothing is read from an Archive that changed on disk, and data read while it changed is
    /// dropped.
    pub(crate) fn start(
        index: &Arc<EntryIndex>,
        id: u32,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        if index.watch().check().is_err() {
            return Self {
                entry: id,
                state: State::ArchiveChanged,
                image: None,
                xml: None,
                xml_arrived: false,
            };
        }
        let wake = Arc::new(wake);
        let image = index
            .entry(id)
            .filter(|entry| Texture::is_header_path(entry.path()))
            .map(|_| {
                let wake = Arc::clone(&wake);
                ImagePreview::start(index, id, move || wake())
            });
        let mut preview = Self {
            entry: id,
            state: State::Failed(None),
            image,
            xml: None,
            xml_arrived: false,
        };
        preview.state = match index.entry(id) {
            None => State::Failed(None),
            Some(entry) if entry.uncompressed_size() > PREVIEW_SIZE_LIMIT => State::TooLarge,
            Some(_) => match spawn(index, id, move || wake()) {
                Spawned::Running(result) => State::Loading(result),
                Spawned::Finished(read) => preview.settle(read),
            },
        };
        preview
    }

    /// Returns the XML preview when the Entry is `CryXml`, known once its start was read.
    #[must_use]
    pub const fn xml(&self) -> Option<&XmlPreview> {
        self.xml.as_ref()
    }

    /// Returns whether the Entry turned out to be `CryXml` since the last call.
    pub(crate) const fn take_xml_arrival(&mut self) -> bool {
        std::mem::replace(&mut self.xml_arrived, false)
    }

    /// Returns the image preview when the Entry is the header of a texture.
    #[must_use]
    pub const fn image(&self) -> Option<&ImagePreview> {
        self.image.as_ref()
    }

    /// Returns the image preview for choosing its mip, when the Entry is the header of a texture.
    pub const fn image_mut(&mut self) -> Option<&mut ImagePreview> {
        self.image.as_mut()
    }

    /// Returns the id of the previewed Entry.
    #[must_use]
    pub const fn entry(&self) -> u32 {
        self.entry
    }

    /// Returns where the preview stands.
    #[must_use]
    pub fn status(&self) -> PreviewStatus<'_> {
        match &self.state {
            State::Loading(_) => PreviewStatus::Loading,
            State::TooLarge => PreviewStatus::TooLarge,
            State::Failed(reason) => PreviewStatus::Failed(reason.as_deref()),
            State::ArchiveChanged => PreviewStatus::ArchiveChanged,
            State::Ready(contents) => PreviewStatus::Ready(contents),
        }
    }

    /// Returns whether the Entry is still being read.
    #[must_use]
    pub const fn is_loading(&self) -> bool {
        matches!(self.state, State::Loading(_))
    }

    /// Returns the data read, or `None` while reading or when nothing was read.
    #[must_use]
    pub const fn contents(&self) -> Option<&PreviewContents> {
        match &self.state {
            State::Ready(contents) => Some(contents),
            _ => None,
        }
    }

    /// Takes the data once it was read and the results of the image preview; returns whether
    /// one arrived.
    pub(crate) fn poll(&mut self) -> bool {
        let imaged = self.image.as_mut().is_some_and(ImagePreview::poll);
        let read = self.poll_data();
        let converted = self.xml.as_mut().is_some_and(XmlPreview::poll);
        read || imaged || converted
    }

    fn poll_data(&mut self) -> bool {
        let State::Loading(result) = &self.state else {
            return false;
        };
        let read = match result.try_recv() {
            Ok(read) => read,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Err(Unread::Failed(None)),
        };
        self.state = self.settle(read);
        true
    }

    fn settle(&mut self, read: ReadResult) -> State {
        match read {
            Ok(Loaded { contents, xml }) => {
                self.xml_arrived = xml.is_some();
                self.xml = xml;
                State::Ready(contents)
            }
            Err(Unread::Failed(reason)) => State::Failed(reason),
            Err(Unread::ArchiveChanged) => State::ArchiveChanged,
        }
    }
}

enum Spawned {
    Running(Receiver<ReadResult>),
    Finished(ReadResult),
}

fn spawn(index: &Arc<EntryIndex>, id: u32, wake: impl Fn() + Send + 'static) -> Spawned {
    let (sender, result) = mpsc::channel();
    let thread_index = Arc::clone(index);
    let spawned = thread::Builder::new()
        .name("preview".to_owned())
        .spawn(move || {
            read(
                &thread_index,
                id,
                |loaded| sender.send(loaded).is_ok(),
                &wake,
            );
        });
    match spawned {
        Ok(_handle) => Spawned::Running(result),
        Err(_ignored) => {
            let mut finished = None;
            read(
                index,
                id,
                |loaded| {
                    finished = Some(loaded);
                    true
                },
                &|| {},
            );
            Spawned::Finished(finished.unwrap_or(Err(Unread::Failed(None))))
        }
    }
}

/// Reads the start of the Entry `id` and hands it to `deliver`, which returns whether the
/// preview still wants it; converts a `CryXml` Entry afterwards. Calls `wake` after each result.
fn read(index: &EntryIndex, id: u32, deliver: impl FnOnce(ReadResult) -> bool, wake: &dyn Fn()) {
    let Some(entry) = index.entry(id) else {
        if deliver(Err(Unread::Failed(None))) {
            wake();
        }
        return;
    };
    let opened = index
        .archive()
        .open_entry(entry)
        .map_err(|error| describe(&error));
    let mut reader = match opened {
        Ok(reader) => reader,
        Err(reason) => {
            let unread = if index.watch().check().is_err() {
                Unread::ArchiveChanged
            } else {
                Unread::Failed(Some(reason))
            };
            if deliver(Err(unread)) {
                wake();
            }
            return;
        }
    };
    let bytes = read_start(&mut reader, entry);
    let read = if index.watch().check().is_err() {
        Err(Unread::ArchiveChanged)
    } else {
        bytes.map_err(|reason| Unread::Failed(Some(reason)))
    };
    let (read, job) = match read {
        Ok(bytes) => {
            let (xml, job) = is_cryxml(&bytes).then(XmlPreview::pending).unzip();
            let start = job.as_ref().map(|_| bytes.clone());
            let contents = PreviewContents::new(bytes, entry.uncompressed_size());
            (Ok(Loaded { contents, xml }), job.zip(start))
        }
        Err(unread) => (Err(unread), None),
    };
    if !deliver(read) {
        return;
    }
    wake();
    if let Some((job, start)) = job {
        job.run(
            index,
            (&start, entry.uncompressed_size()),
            &mut reader,
            wake,
        );
    }
}

fn read_start(reader: &mut impl Read, entry: &Entry) -> Result<Vec<u8>, String> {
    let limit = u64::try_from(PREVIEW_READ_LIMIT).unwrap_or(u64::MAX);
    let capacity = usize::try_from(entry.uncompressed_size().min(limit)).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    reader
        .by_ref()
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|error| describe(&error))?;
    Ok(bytes)
}
