// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use p4kx_extraction::{CancelToken, LoadedTexture, TextureLoadError, TextureLoader};

use super::stop::StoppableTextures;
use super::{Observed, Shared};

/// A writer that cancels the export while it takes the first chunk.
struct Cancelling(CancelToken);

impl Write for Cancelling {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.cancel();
        Ok(buffer.len().min(2))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_write_after_the_cancel_fails_instead_of_being_retried_forever() {
    let cancel = CancelToken::new();
    let shared = Shared::default();
    let mut observed = Observed {
        inner: Cancelling(cancel.clone()),
        cancel: &cancel,
        written: &shared.done,
    };

    let result = observed.write_all(b"glTF binary");

    let error = result.expect_err("the second chunk meets the cancel");
    assert_eq!(error.kind(), io::ErrorKind::Other, "no retry by write_all");
    assert_eq!(shared.progress().done, 2, "only the first chunk counted");
}

struct FourByFour;

impl TextureLoader for FourByFour {
    fn load(&self, _path: &str, _mip: u32) -> Result<LoadedTexture, TextureLoadError> {
        Err(TextureLoadError::NotFound)
    }

    fn dimensions(&self, _path: &str, _mip: u32) -> Result<Option<(u32, u32)>, TextureLoadError> {
        Ok(Some((4, 4)))
    }
}

#[test]
fn header_reads_stop_after_the_cancel() {
    let cancel = CancelToken::new();
    let inner = FourByFour;
    let textures = StoppableTextures {
        textures: &inner,
        cancel: &cancel,
    };

    assert!(matches!(textures.dimensions("a.dds", 0), Ok(Some((4, 4)))));
    cancel.cancel();

    assert!(textures.dimensions("a.dds", 0).is_err(), "stopped");
}

#[test]
fn an_export_that_is_not_cancelled_counts_what_it_wrote() {
    let cancel = CancelToken::new();
    let shared = Shared::default();
    let mut observed = Observed {
        inner: Vec::new(),
        cancel: &cancel,
        written: &shared.done,
    };

    observed.write_all(b"abcd").expect("a vector takes it");

    assert_eq!(shared.progress().done, 4);
}
