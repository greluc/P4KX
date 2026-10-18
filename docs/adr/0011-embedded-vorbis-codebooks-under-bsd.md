# Embedded Vorbis codebook libraries under their BSD licence

Wwise Vorbis sounds refer to their codebooks by id into a codebook library instead of storing
them, so rebuilding a playable Ogg Vorbis file needs that library. The only source is ww2ogg's
`packed_codebooks.bin` (libvorbis) and `packed_codebooks_aoTuV_603.bin` (aoTuV 6.03), licensed
BSD-3-Clause rather than MIT. `p4kx-audio` embeds both files unchanged with `include_bytes!`;
the rebuild, validation and fallback code is our own. Their copyright notices and licence text
live in `THIRD-PARTY-NOTICES` at the repository root, which ships in the portable ZIP and the MSI
next to the licences and is named in the README. Decided by the owner.

## Considered Options

- The `ww2ogg` crate as a dependency: least work, but couples the Vorbis path to a single-release
  crate whose handling of untrusted input we do not control (see ADR-0001).
- Inline codebooks only: no foreign data, but every Star Citizen sound refers to library ids.
- Codebook files supplied by the user: no foreign data in the repository, but no Ogg export out of
  the box.
