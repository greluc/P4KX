// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};
use std::path::Path;
use std::sync::{Mutex, Once, PoisonError};
use std::thread::{self, JoinHandle};

use p4kx_extraction::{
    ConversionError, ExtractionError, QueryExportError, WriteRefused, game_is_running,
    protect_archive_folder,
};

const GAME_CHECK_VARIABLE: &str = "P4KX_GAME_CHECK";

static WARNED: Once = Once::new();
static CHECK: Mutex<Option<JoinHandle<bool>>> = Mutex::new(None);

/// Returns whether the look for the running game is switched off with `P4KX_GAME_CHECK=off`, as
/// the test suite does so that a game running on the developer's machine changes no output.
fn game_check_off() -> bool {
    std::env::var(GAME_CHECK_VARIABLE).is_ok_and(|value| value.eq_ignore_ascii_case("off"))
}

/// Starts looking for the running game in the background, so that the first Archive opened does
/// not wait for it.
pub(crate) fn start_game_check() {
    if game_check_off() {
        return;
    }
    let spawned = thread::Builder::new()
        .name("game-check".to_owned())
        .spawn(game_is_running);
    if let Ok(handle) = spawned {
        *CHECK.lock().unwrap_or_else(PoisonError::into_inner) = Some(handle);
    }
}

/// Protects the installation around the Archive at `path` and warns once per run when the game is
/// running.
pub(crate) fn on_archive_open(path: &Path) {
    protect_archive_folder(path);
    if game_check_off() {
        return;
    }
    WARNED.call_once(|| {
        let started = CHECK.lock().unwrap_or_else(PoisonError::into_inner).take();
        let running = started.map_or_else(game_is_running, |handle| handle.join().unwrap_or(false));
        if running {
            let _ignored = writeln!(
                io::stderr().lock(),
                "warning: StarCitizen.exe is running; use this tool with the game closed"
            );
        }
    });
}

/// Returns whether `error` is a write that was refused because the target lies inside a Star
/// Citizen installation.
pub(crate) fn is_refusal(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause.downcast_ref::<WriteRefused>().is_some()
            || matches!(
                cause.downcast_ref::<ConversionError>(),
                Some(ConversionError::Installation(_))
            )
            || matches!(
                cause.downcast_ref::<p4kx_blender::PackageError>(),
                Some(p4kx_blender::PackageError::Refused(_))
            )
            || matches!(
                cause.downcast_ref::<ExtractionError>(),
                Some(ExtractionError::Installation(_))
            )
            || matches!(
                cause.downcast_ref::<QueryExportError>(),
                Some(QueryExportError::Installation(_))
            )
            || cause
                .downcast_ref::<io::Error>()
                .and_then(io::Error::get_ref)
                .and_then(|inner| inner.downcast_ref::<WriteRefused>())
                .is_some()
    })
}

/// Refuses `target`, when there is one, when it lies inside a Star Citizen installation.
pub(crate) fn ensure_writable_if(target: Option<&Path>) -> io::Result<()> {
    target.map_or(Ok(()), ensure_writable)
}

/// Refuses `target` when it lies inside a Star Citizen installation.
pub(crate) fn ensure_writable(target: &Path) -> io::Result<()> {
    p4kx_extraction::ensure_writable(target)
        .map_err(|refused| io::Error::new(io::ErrorKind::PermissionDenied, refused))
}
