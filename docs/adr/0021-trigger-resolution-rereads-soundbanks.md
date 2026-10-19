# Trigger resolution re-reads soundbanks and bounds the HIRC walk

Resolving an audio trigger follows each of its Wwise events through the `HIRC` section of every
soundbank holding it: Event, then its Play actions (action types `0x04xx`; stop, pause, set-state
and the other actions lead to no sound and are skipped), then random, sequence, switch and blend
containers, to Sound objects. Each Sound reached is a sound variant with its container path from
the Event object to the Sound object. The audio index of ADR-0018 keeps no hierarchy; the
soundbanks holding a trigger's events are read again from the Archive under the same 16 MiB cap,
in parallel, followed and dropped, so at most one capped soundbank per rayon thread is held. The
index adds only the WEMs every soundbank embeds (`DIDX`: ID, offset in `DATA`, length), so a
variant's WEM is located in the soundbank that holds the hierarchy first, then in any other
soundbank, without reading that one.

`p4kx-audio` decodes containers to their children for bank version 154 only, the version every
soundbank of the live Archive has; the layout of the node base parameters (effects, metadata,
properties, positioning with 3D automation, auxiliary sends, state properties and groups, RTPC
curves) was checked against all 2386 live soundbanks, where every one of the 72 000 containers and
250 000 sounds decodes to exactly its last byte. A container must use its whole body; one that
does not is malformed, and a container of another version stays raw. Both are broken links, so a
layout change of a later Wwise version shows up as broken links, not as wrong children.

The hierarchy is untrusted input. A path that refers back to an object on itself is a cycle and is
cut there; a path is at most 32 objects deep, at most the first 1024 children of a container are
followed, and at most 16 384 objects are followed from one event in one soundbank. Each limit
that cuts the walk is reported as a broken link at the object where it cut; the variants found
before it are kept.

In the live data, 99 % of the Sound objects whose media is held in memory (storage 0) have it in no
soundbank's `DIDX`: the game loads them as loose `Data\Sounds\wwise\Media\<id>.wem` Entries. Such
a variant is listed without a location rather than as a broken link; locating loose WEMs is #143.

## Considered Options

- Keep the whole hierarchy of every soundbank in the audio index: one read per Archive, but
  hundreds of thousands of objects held for the lifetime of the Archive, while resolving one
  trigger touches a handful of soundbanks.
- A global visited set per event instead of the path: stops a shared node from being reached
  twice, but a sound reached through two switch states is two variants with two container paths;
  the object budget bounds the work instead.
- Index every `HIRC` object ID of every soundbank to follow an action into another soundbank: of
  58 000 live Play actions 554 aim at no object of their own soundbank, and 507 of those at no
  object of any soundbank; they are reported as broken links.
