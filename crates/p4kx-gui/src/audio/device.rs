use std::num::NonZero;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use rodio::{ChannelCount, DeviceSinkBuilder, SampleRate, Source};

use super::{AudioOutput, FailureReporter, SoundSource, StreamGuard};
use crate::model::describe;

/// The default audio device of the system, opened through rodio on a thread of its own for
/// every sound played; rodio converts the sound to the device's rate and channels.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeviceOutput;

impl AudioOutput for DeviceOutput {
    fn start(&self, source: SoundSource) -> StreamGuard {
        let (guard, closed) = mpsc::channel::<()>();
        let reporter = source.reporter();
        let thread_reporter = reporter.clone();
        let spawned = thread::Builder::new()
            .name("audio-output".to_owned())
            .spawn(move || play(source, &thread_reporter, &closed));
        if let Err(error) = spawned {
            reporter.report(describe(&error));
        }
        StreamGuard::new(guard)
    }
}

fn play(source: SoundSource, reporter: &FailureReporter, closed: &Receiver<()>) {
    let runtime = reporter.clone();
    let opened = DeviceSinkBuilder::from_default_device().and_then(|builder| {
        builder
            .with_error_callback(move |error| runtime.report(describe(&error)))
            .open_sink_or_fallback()
    });
    let mut sink = match opened {
        Ok(sink) => sink,
        Err(error) => {
            reporter.report(describe(&error));
            return;
        }
    };
    sink.log_on_drop(false);
    sink.mixer().add(Feed(source));
    let _closed = closed.recv();
}

/// A [`SoundSource`] as a rodio source.
struct Feed(SoundSource);

impl Iterator for Feed {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_sample()
    }
}

impl Source for Feed {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        NonZero::new(self.0.channels()).unwrap_or(NonZero::<u16>::MIN)
    }

    fn sample_rate(&self) -> SampleRate {
        NonZero::new(self.0.sample_rate()).unwrap_or(NonZero::<u32>::MIN)
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
