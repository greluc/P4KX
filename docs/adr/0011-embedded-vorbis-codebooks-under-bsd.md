# Embedded Vorbis codebook libraries under their BSD licence

Wwise Vorbis sounds refer to their codebooks by id into a codebook library instead of storing
them, so rebuilding a playable Ogg Vorbis file needs that library. The only source is ww2ogg's
`packed_codebooks.bin` (libvorbis) and `packed_codebooks_aoTuV_603.bin` (aoTuV 6.03), licensed
BSD-3-Clause rather than MIT. `p4kx-audio` embeds both files unchanged with `include_bytes!`;
the rebuild, validation and fallback code is our own. Their copyright notices and licence text
live in `THIRD-PARTY-NOTICES` at the repository root, which ships in the portable ZIP and the MSI
next to the licences and is named in the README. Decided by the owner.

Which library a sound needs is not recorded in the WEM, so the rebuilt headers are validated: the
setup is checked like libvorbis checks it, and the first 16 audio packets are decoded with lewton
(MIT OR Apache-2.0), which makes lewton a runtime dependency of `p4kx-audio`. It is the only
pure-Rust Vorbis decoder that decodes packets without an Ogg container, and #93 needs a decoder
anyway. lewton runs inside `catch_unwind`, so a panic on hostile input rejects the candidate; this
relies on the workspace keeping `panic = "unwind"`, the default, in every profile.

Validating the whole stream instead of the first 16 packets (as StarBreaker does) was measured
and is not adopted: decoding 6,161 live Vorbis WEMs to their end costs 111 s of CPU time against
16 s for rebuilding them, about eight times as much, and none of them fails after its first 16
packets (`docs/audio-survey.md`). A wrong codebook library shows in the first packets; a later
bad packet is still a typed error when the sound is converted.

## Considered Options

- The `ww2ogg` crate as a dependency: least work, but couples the Vorbis path to a single-release
  crate whose handling of untrusted input we do not control (see ADR-0001).
- Inline codebooks only: no foreign data, but every sampled Star Citizen sound refers to library
  ids.
- Codebook files supplied by the user: no foreign data in the repository, but no Ogg export out of
  the box.
- Validation by setup checks alone, without a decoder: no runtime decoder, but codebooks of the
  wrong library can pass every structural check.
