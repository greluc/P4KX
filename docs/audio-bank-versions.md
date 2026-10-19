# Soundbank versions and the object layouts p4kx-audio decodes

`BKHD` states the bank version. `Soundbank::parse` reads banks of version 49 to 65,535; it decodes
a `HIRC` object only for the versions whose layout it knows, and keeps every other object's body
raw.

| Object | Decoded for versions | Layout after the object ID |
| --- | --- | --- |
| Sound | 150 to 154 | plug-in ID, storage byte, source ID, `cacheID` (from 151 on, see #295), in-memory media size |
| Action | 49 to 154 | action type `u16`, target ID |
| Event | 49 to 154 | action count (4 bytes up to 122, a variable-length integer from 123), action IDs |
| Random, sequence, switch, blend container | 154 | node base, then the children |
| Any other object | never | raw |

Above version 154 the layouts change (from version 155 an Event starts with a byte, an instance
limit and a cooldown before its action count, and more follows from 168), so a Sound, Action,
Event or container of such a bank is `HircValue::Malformed` with
`BankError::UnsupportedObjectLayout { kind, version }`; its body stays available through
`Soundbank::object_body`, the bank's WEMs stay readable, and the audio index reports the object as
a broken link. Below its first version a Sound or container stays `HircValue::Raw`.

Type numbers: up to version 126 the types from `0x10` on are a feedback bus (`0x10`), a feedback
node (`0x11`), effect share set, custom effect, auxiliary bus, LFO, envelope and audio device
(`0x12` to `0x17`); from version 127 on they are effect share set (`0x10`) to time modulator
(`0x16`). `HircKind::of(version, byte)` maps by version; the feedback types are
`HircKind::Other`. Version 127 follows the new numbering (no bank of version 127 is known).

Bank versions of the Archives, measured on 2026-10-20 by
`real_archive/soundbank.rs::every_real_soundbank_parses_with_its_wems_and_hirc_objects`:
see the pull request of #296; LIVE, PTU and TECH-PREVIEW hold only version 154.
