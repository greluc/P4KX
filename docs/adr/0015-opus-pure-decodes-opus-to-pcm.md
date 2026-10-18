# opus-pure decodes Opus WEMs to PCM

WAV export of Opus WEMs decodes their packets with opus-pure 0.2.2 (BSD-3-Clause), a pure-Rust
port of libopus without dependencies, and streams the 16-bit samples through the WAV writer of the
other codecs. Ogg export needs no decoder: the packets keep their bytes and get an `OpusHead`, an
`OpusTags` packet and granule positions from our own Ogg page writer. Both read the three Wwise
framings as vgmstream does: Wwise Opus (`0x3041`) with the packet sizes in the `seek` chunk, a
complete Ogg Opus stream in the `data` chunk (`0x3040`, whose pages are read with their CRC
checked) and Switch Opus (`0x3039`) with an 8-byte header per packet and a pre-skip of an eighth
of the first packet. A multichannel Wwise Opus sound gets the stream and coupled counts and the
channel mapping vgmstream derives from the Wwise layout; the WAV holds the channels in the speaker
order of the WEM's channel mask, as vgmstream gets them through FFmpeg. Two deviations from
vgmstream: a Wwise mapping of 255 gets channel mapping family 255 with the identity mapping
instead of family 1, which RFC 7845 limits to 8 channels and which would reorder the channels,
and Switch Opus is read for one and two channels only, as vgmstream builds no mapping for more.
The `meta` chunk some Gears 5 WEMs carry before their Ogg data is not read. An embedded Ogg
stream keeps its own `OpusHead` and `OpusTags`. Packets are capped at 64 KiB, a multistream
packet is split by its self-delimited framing with every length checked, and the decoder runs
inside `catch_unwind` like lewton (ADR-0013). Star Citizen's LIVE and PTU Archives hold no Opus
WEM, so the synthetic WEMs of the testkit, encoded with the same crate, are the only test data.
Its notice lives in `THIRD-PARTY-NOTICES`. Decided for issue #94.

## Considered Options

Measured against the 12 RFC 8251 test vectors at 48 kHz stereo, with 100,000 random and
bit-flipped packets for robustness:

- opus-pure 0.2.2: within one LSB of the reference output on every vector (99 to 116 dB SNR,
  bit-exact on three), no panic, 2.0 s for all vectors. `unsafe` only in SIMD kernels; fuzz
  targets and a robustness suite in its repository. The risk is its single maintainer and a fork
  history of a few months, released in September 2026; the decoder sits behind `Wem::write_wav`,
  so it can be swapped without an API change.
- rusty-opus 1.0.2 (BSD-3-Clause), the crate opus-pure was forked from: as exact and 1.6 s, but
  the release was two days old, and its repository advertises install counts in its commits.
- opus-decoder 0.1.1 (MIT OR Apache-2.0, `forbid(unsafe_code)`): the licence and safety fit
  best, but it differs from the reference by up to 242 LSB (43 to 58 dB) on four vectors and took
  60.8 s; no release since March 2026.
- opus-rs 0.1.37 (BSD-3-Clause): fails most SILK and hybrid vectors through its decoder API.
- opuscule 0.2.1 (MPL-2.0): its repository is gone.
- symphonia: its Opus crate is an empty stub.
- libopus through `audiopus` or `opus`: the reference, but C code on untrusted input against
  ADR-0001, and `audiopus_sys` is unmaintained (RUSTSEC-2026-0150).
