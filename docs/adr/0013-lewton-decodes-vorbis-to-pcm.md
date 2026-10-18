# lewton decodes Wwise Vorbis to PCM for WAV export

WAV export of Wwise Vorbis sounds rebuilds the Vorbis headers as for Ogg export (ADR-0011),
decodes the rebuilt audio packets one by one with lewton (MIT OR Apache-2.0), the decoder that
already validates the codebooks, and streams the 16-bit samples through the WAV writer of the
other codecs. No Ogg container is built in between. The channels stay in the order of the
stream, which Wwise writes in the speaker order of the WEM's channel mask, as vgmstream reads
them, and the WAV carries that mask. lewton runs inside `catch_unwind`, and it only ever sees
headers our rebuild wrote, with codebooks of at most 16383 entries and 15 dimensions, and audio
packets below 64 KiB. Decided for issue #93.

## Considered Options

- symphonia-codec-vorbis 0.6.1 (MPL-2.0): actively maintained and about 2.5 times faster (0.43
  to 0.60 s against 1.10 to 1.40 s for a 392-second stereo sound of the LIVE Archive), but it
  undoes channel coupling in the order the steps are stored, while the Vorbis specification,
  libvorbis and lewton go from the last step to the first, which changes the result whenever
  two steps share a channel. All four 4-channel sounds in a sample of 2019 WEMs of the LIVE
  Archive decode differently from lewton, by up to 42 % of full scale. It also rejects residues
  that use codebook 0, which the specification allows, sorts the channels into its own speaker
  order, and adds an MPL-2.0 dependency tree with a complex number crate to a format crate.
- Writing the Ogg file and decoding it through lewton's Ogg reader: reuses a tested path, but
  builds the whole Ogg in memory for nothing and pulls the `ogg` crate into the runtime.
- The C libvorbis: the reference decoder, but C code on untrusted input and a C toolchain in the
  build, against ADR-0001.
