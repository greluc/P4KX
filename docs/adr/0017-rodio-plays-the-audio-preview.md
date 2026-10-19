# rodio plays the audio preview

The Audio tab of the preview plays WEMs through rodio 0.23 (MIT OR Apache-2.0) with only its
`playback` feature, which brings cpal 0.18 (Apache-2.0) and no decoders. The decoding stays ours:
a worker thread per preview reads the Entry and decodes it through `Wem::write_wav` into a writer
that keeps the 16-bit samples, so every codec `p4kx-audio` decodes plays, and one it cannot is
named. The worker stops on a cancel flag that dropping the preview sets, also while it reads the
Entry. The decoded sound is held in memory up to 256 MiB of PCM, about 23 minutes of stereo at
48 kHz; a longer sound is not decoded, and a WEM that states a longer length is refused before
decoding. Play, pause, seek, volume and the position live in atomics the window and the audio
thread share; the window never waits for the audio thread. Each Play that needs a stream opens
the default device on a thread of its own, so opening a slow or missing device never blocks the
window, and the error callback and a failed opening report to the window, which shows the reason
instead of playing. The output is a trait, so tests drive the controls through a fake output
without an audio device. rodio converts the sample rate and the channel count to the device's;
cpal needs the ALSA headers on Linux, which CI installs. Decided for issue #98.

## Considered Options

- cpal 0.18 alone: the smallest dependency, but it plays only configurations the device offers,
  and in WASAPI shared mode that is usually the mix format, so the preview would need its own
  sample-rate and channel conversion for 22.05, 32 and 44.1 kHz sounds on a 48 kHz device.
- rodio's own `Player` with pause, volume and seek: less code, but seeking needs a seekable
  source and the state is inside rodio, so the controls could not be tested without a device.
- kira and other game audio engines: mixers, tweens and effects the preview does not need, and a
  larger dependency tree.
- Streaming the decoded samples instead of holding them: no memory cap needed, but seeking a
  Vorbis WEM would mean decoding again from the start, and the decoders write a whole WAV.
