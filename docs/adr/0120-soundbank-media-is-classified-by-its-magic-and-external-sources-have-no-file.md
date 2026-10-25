# Soundbank media is classified by its magic, and external sources have no file

Not every `DIDX` item of a soundbank is a WEM, and not every Sound object has a file. This
reader tells them apart instead of reporting a generic "not a WEM".

**Media class.** `MediaClass::of` reads the first bytes of an item: `RIFF` or `RIFX` followed by
`WAVE` is a `wem`; `MThd` (a standard MIDI file) or `RIFF` with the form `RMID` is `midi`; `PLUG`
is `impulse`, Wwise plug-in data such as a convolution reverb's impulse response; anything else,
including an item shorter than four bytes, is `unknown`. `midi` and `impulse` are non-sound
classes: `Wem::parse` returns `AudioError::MidiMedia` and `AudioError::PluginMedia` for them,
extraction writes them unchanged, and the listings (`p4kx bnk list`, the Audio tab) show the class
and no codec. `unknown` keeps `AudioError::NotWem`. The names `wem`, `midi`, `impulse` and
`unknown` are the stable `class` values of `p4kx bnk list --json`. vgmstream also knows ADM3
media; no live soundbank holds any, so it stays `unknown` until one does.

**Sources without a file.** A Sound object whose plug-in ID has the source-plug-in type (`&
0xF == 2`, e.g. Silence `0x00650002`, Tone, Synth) generates its sound, and the External Source
plug-in `0x00080001` is resolved by the game at runtime. Neither has a WEM in any soundbank or
loose Entry, so `SoundVariant::wem_id` is `None` and `SourceKind` is `Plugin` or `External`; they
are not located, not counted as not located, and not reported as missing media. The MIDI plug-in
`0x00100001` has a file: its MIDI item lies in a soundbank and is located like a WEM, with
`SourceKind::Midi`; a preview cannot play it.

**Live Archive.** The 2386 live soundbanks (all version 154) embed 2399 media items: 2285 `wem`
(917 of them prefetched starts), 114 `impulse`, no `midi` and no `unknown`. Of their 249 522 Sound
objects, 5276 use a source plug-in and 50 the External Source plug-in; none uses the MIDI plug-in.
