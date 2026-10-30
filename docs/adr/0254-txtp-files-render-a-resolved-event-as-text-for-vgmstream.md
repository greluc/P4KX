# TXTP files render a resolved Wwise event as text for vgmstream

Decided for issue #307, on top of ADR-0115 and ADR-0158 (the event resolution and the `HIRC`
decoding) and ADR-0250 (the object kinds). The format is that of vgmstream's `doc/TXTP.md`.

## Decision

- **Text only.** `p4kx wwise extract --txtp` writes `.txtp` text files next to the exported
  sounds. P4KX neither bundles nor runs vgmstream, and the syntax was read from its documentation;
  nothing of wwiser or vgmstream is copied.
- **No second parser.** `p4kx_assembly::audio::write_txtp` works on the variants of a resolved
  trigger (`SoundVariant::path`), not on the bank. Variants of one Wwise event come in depth-first
  order, so the tree of objects is rebuilt from their paths: a path step equal to the last open
  child is the same object. A Sound object is a leaf; a music track holds its sources as leaves.
- **One file per event** that leads to sounds, named after the event (`<event>.txtp`) by the
  rule of the sound files: ASCII letters, digits, `_` and `-`, at most 120 bytes, a name taken
  ignoring case gets `_2`, `_3`, …. The file lines name the exported files, which carry the same
  safe names and lie in the same folder, so every path in a TXTP is a bare file name. The rule is
  the safe subset of the portable file-name rule of issue #861, so it holds on Linux and Windows
  alike whether or not that rule is shared code by then.
- **How objects combine.**

  | Object | TXTP |
  |---|---|
  | Event (its actions), blend container, music segment, music track | `group = -L<n> #@layer-v` |
  | Sequence container, music playlist | `group = -S<n>` |
  | Random container, switch container, music switch | `group = -R<n>>1` |

  A group of one child is the child. A random or switch container is settled at run time in the
  game; the file selects its first child, and the `group` line says how to select another.
- **Loops come from the WEM.** The loop of the WEM (`smpl` chunk, issue #95) is written as
  `#I <start> <end>` in frames, the values the sound file carries as `smpl` or `LOOPSTART` too.
  `ExportedSound` gains `loop_points` and `prefetch_only` for this.
- **No guess where a file is missing.** An event with a sound that has no whole file (plug-in
  or external source, a WEM no soundbank holds, only the prefetched start of a streamed WEM) gets
  no TXTP and is listed with the reason in the report; the exit code is that of the sound export,
  which already lists such sounds as failed.
- **Names from the Archive stay in comments.** The header comment holds the event and trigger
  name with control characters replaced, so a name cannot start a TXTP line of its own.

## Consequences

- Music timing (clip positions, trims, transitions, the loop counts of containers and playlist
  items) is not reproduced; a music event plays its tracks as layers. The map above is a faithful
  rendition for events of sounds and containers and an approximation for music.
- `ExportedSound` has two new public fields; constructing one outside the crate needs them.
- The JSON of `wwise extract` has an optional `txtp` object only with `--txtp`.

## Considered Options

- One TXTP per selectable child of a random container, as wwiser can write them: many files per
  event, and the choice is a run-time one; not done, the `>1` in the file is edited by hand.
- Re-reading the bank to walk the objects again for the TXTP: a second reader of the `HIRC`
  section; not done.
- Writing a silent `?.` entry for a source without a file: plays a rendition that differs from
  the game without saying so; not done.
