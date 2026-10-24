// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_extraction::model::{LodSelection, companion_file_names};
use p4kx_extraction::{AnimationChoice, AnimationSet, Entry, ModelSource, PathIndex};

use super::camera::Camera;
use super::scene::{Scene, SceneError};
use crate::model::describe;
use crate::{EntryIndex, ModelInfo};

/// The largest model file or geometry companion, in bytes, the viewer reads.
pub const MODEL_FILE_LIMIT: u64 = 256 * 1024 * 1024;

/// Returns whether `path` names a model file the Model tab shows: a `.cgf`, `.cga`, `.skin` or
/// `.chr`.
#[must_use]
pub fn is_model_path(path: &str) -> bool {
    path.rsplit_once('.').is_some_and(|(_, extension)| {
        ["cgf", "cga", "skin", "chr"]
            .iter()
            .any(|known| extension.eq_ignore_ascii_case(known))
    })
}

/// Where the model preview of a model Entry stands.
#[derive(Debug, Clone, Copy)]
pub enum ModelStatus<'a> {
    /// The model and its geometry companion are being read and flattened.
    Loading,
    /// The model could not be read, for the reason given, or reading stopped without a result.
    Failed(Option<&'a str>),
    /// The Archive changed on disk, so the model was not read or its data was dropped.
    ArchiveChanged,
    /// The model file or its companion is this many bytes, above [`MODEL_FILE_LIMIT`].
    FileTooLarge(u64),
    /// The model was read but has nothing the viewer can show or more than it takes.
    Unshowable(SceneError),
    /// The model was read and flattened.
    Ready(&'a Arc<Scene>),
}

#[derive(Debug)]
enum Event {
    Failed(Option<String>),
    ArchiveChanged,
    FileTooLarge(u64),
    Read(Box<(ModelInfo, Result<Scene, SceneError>)>),
}

#[derive(Debug)]
enum State {
    Loading(Receiver<Event>),
    Failed(Option<String>),
    ArchiveChanged,
    FileTooLarge(u64),
    Unshowable(SceneError),
    Ready(Arc<Scene>),
}

/// The model preview of a model Entry: the model read and flattened on a background thread, and
/// the camera that looks at it.
#[derive(Debug)]
pub struct ModelPreview {
    dropped: Arc<AtomicBool>,
    state: State,
    info: Option<Arc<ModelInfo>>,
    camera: Option<Camera>,
}

impl ModelPreview {
    /// Starts reading the model Entry `id` of `index` on a background thread, calling `wake`
    /// from it once the result is ready.
    pub(crate) fn start(
        index: &Arc<EntryIndex>,
        id: u32,
        wake: impl Fn() + Send + 'static,
    ) -> Self {
        let (sender, events) = mpsc::channel();
        let dropped = Arc::new(AtomicBool::new(false));
        let thread_index = Arc::clone(index);
        let thread_dropped = Arc::clone(&dropped);
        let spawned = thread::Builder::new()
            .name("model-preview".to_owned())
            .spawn(move || {
                if let Some(event) = load(&thread_index, id, &thread_dropped)
                    && !thread_dropped.load(Ordering::Acquire)
                    && sender.send(event).is_ok()
                {
                    wake();
                }
            });
        let state = match spawned {
            Ok(_handle) => State::Loading(events),
            Err(_ignored) => State::Failed(None),
        };
        Self {
            dropped,
            state,
            info: None,
            camera: None,
        }
    }

    /// Returns the facts of the model once it was read, also when the viewer cannot show it.
    #[must_use]
    pub const fn info(&self) -> Option<&Arc<ModelInfo>> {
        self.info.as_ref()
    }

    /// Returns where the preview stands.
    #[must_use]
    pub fn status(&self) -> ModelStatus<'_> {
        match &self.state {
            State::Loading(_) => ModelStatus::Loading,
            State::Failed(reason) => ModelStatus::Failed(reason.as_deref()),
            State::ArchiveChanged => ModelStatus::ArchiveChanged,
            State::FileTooLarge(size) => ModelStatus::FileTooLarge(*size),
            State::Unshowable(error) => ModelStatus::Unshowable(*error),
            State::Ready(scene) => ModelStatus::Ready(scene),
        }
    }

    /// Returns whether the model is still being read.
    #[must_use]
    pub const fn is_loading(&self) -> bool {
        matches!(self.state, State::Loading(_))
    }

    /// Returns the flattened model once it was read.
    #[must_use]
    pub const fn scene(&self) -> Option<&Arc<Scene>> {
        match &self.state {
            State::Ready(scene) => Some(scene),
            _ => None,
        }
    }

    /// Returns the camera once the model was read.
    #[must_use]
    pub const fn camera(&self) -> Option<&Camera> {
        self.camera.as_ref()
    }

    /// Returns the camera for moving it once the model was read.
    pub const fn camera_mut(&mut self) -> Option<&mut Camera> {
        self.camera.as_mut()
    }

    /// Takes the result the background thread sent; returns whether it arrived.
    pub(crate) fn poll(&mut self) -> bool {
        let State::Loading(events) = &self.state else {
            return false;
        };
        let event = match events.try_recv() {
            Ok(event) => event,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Event::Failed(None),
        };
        self.state = match event {
            Event::Failed(reason) => State::Failed(reason),
            Event::ArchiveChanged => State::ArchiveChanged,
            Event::FileTooLarge(size) => State::FileTooLarge(size),
            Event::Read(read) => {
                let (info, scene) = *read;
                self.info = Some(Arc::new(info));
                match scene {
                    Ok(scene) => {
                        self.camera = Some(Camera::framing(scene.bounds()));
                        State::Ready(Arc::new(scene))
                    }
                    Err(error) => State::Unshowable(error),
                }
            }
        };
        true
    }
}

impl Drop for ModelPreview {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

fn load(index: &EntryIndex, id: u32, stopped: &AtomicBool) -> Option<Event> {
    let stop = || stopped.load(Ordering::Acquire);
    let Some(entry) = index.entry(id) else {
        return Some(Event::Failed(None));
    };
    if index.watch().check().is_err() {
        return Some(Event::ArchiveChanged);
    }
    let paths = index.path_index();
    let oversized = oversized_file(&paths, entry);
    if let Some(size) = oversized {
        return Some(Event::FileTooLarge(size));
    }
    if stop() {
        return None;
    }
    let source = match ModelSource::from_index_with(&paths, entry.path(), LodSelection::All)
        .or_else(|_| ModelSource::from_index_with(&paths, entry.path(), LodSelection::Only(0)))
    {
        Ok(source) => source,
        Err(error) => {
            return Some(if index.watch().check().is_err() {
                Event::ArchiveChanged
            } else {
                Event::Failed(Some(reason(&error, entry.path())))
            });
        }
    };
    if stop() {
        return None;
    }
    let animations = AnimationSet::from_archive(&paths, entry.path(), &AnimationChoice::All, &[])
        .ok()
        .map(|set| set.animations.len());
    let info = ModelInfo::of(source.model()).with_animations(animations);
    if stop() {
        return None;
    }
    let scene = Scene::from_model(source.model());
    if stop() {
        return None;
    }
    if index.watch().check().is_err() {
        return Some(Event::ArchiveChanged);
    }
    Some(Event::Read(Box::new((info, scene))))
}

fn oversized_file(paths: &PathIndex<'_>, entry: &Entry) -> Option<u64> {
    let path = entry.path();
    companion_file_names(path)
        .iter()
        .filter_map(|companion| paths.find(companion))
        .chain(Some(entry))
        .map(Entry::uncompressed_size)
        .find(|&size| size > MODEL_FILE_LIMIT)
}

fn reason(error: &dyn Error, path: &str) -> String {
    let described = describe(error);
    described
        .strip_prefix(path)
        .and_then(|rest| rest.strip_prefix(": "))
        .map_or_else(|| described.clone(), str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{is_model_path, reason};

    #[test]
    fn model_files_are_recognised_by_extension_ignoring_case() {
        for path in [
            "Data\\Objects\\ship.cgf",
            "Data\\Objects\\ship.CGA",
            "data/objects/gun.skin",
        ] {
            assert!(is_model_path(path), "{path}");
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("{0}")]
    struct Named(String);

    #[test]
    fn the_reason_does_not_repeat_the_path_the_message_names() {
        let error = Named("Data\\Objects\\a.cgf: not a chunk file".to_owned());

        assert_eq!(
            reason(&error, "Data\\Objects\\a.cgf"),
            "not a chunk file",
            "path dropped"
        );
        assert_eq!(
            reason(&error, "Data\\Objects\\b.cgf"),
            "Data\\Objects\\a.cgf: not a chunk file",
            "another path stays"
        );
    }

    #[test]
    fn companions_and_other_files_are_not_model_files() {
        for path in ["ship.cgfm", "ship.skinm", "ship.dds", "cgf", "ship.cgf.xml"] {
            assert!(!is_model_path(path), "{path}");
        }
    }
}
