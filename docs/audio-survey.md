# Survey of the soundbanks and WEMs of real Archives

Measured on 2026-10-20 with the Archives of the LIVE, PTU and TECH-PREVIEW Channels, release
build, by `real_archive/audio_survey.rs` and `real_archive/vorbis_cost.rs` of `p4kx-extraction`
(`#[ignore]`d; run them as `CLAUDE.md` describes). The survey walks the sections of every `.bnk`
Entry itself, so its counts do not depend on what the parsers tolerate; it also walks the RIFF
chunks of every WEM embedded in a bank and of every loose `.wem` Entry.

| | LIVE | PTU | TECH-PREVIEW |
| --- | --- | --- | --- |
| Soundbanks | 2,386 | 2,376 | 2,401 |
| Rejected by `Soundbank::parse` | 0 | 0 | 0 |
| Largest soundbank | 5,418,378 bytes (`SSAM_CRUS_StarFighter.bnk`) | same | same |
| Soundbanks above 16 MiB | 0 | 0 | 0 |
| Bank versions | 154 (2,386) | 154 (2,376) | 154 (2,401) |
| Bytes after the last `HIRC` object | 0 | 0 | 0 |
| Repeated `DIDX` IDs | 0 | 0 | 0 |
| `DIDX` without `DATA` | 0 | 0 | 0 |
| Non-UTF-8 `STID` names | 0 | 0 | 0 |
| WEMs surveyed (embedded and loose) | 119,345 | 118,881 | 120,617 |
| Format tags | `0xFFFF` (all) | `0xFFFF` (all) | `0xFFFF` (all) |
| Chunks after `data` | 0 | 0 | 0 |
| Odd-sized chunks before the last | 0 | 0 | 0 |

The ten largest soundbanks are between 2.2 and 5.4 MB. Every WEM is Wwise Vorbis, so no real WEM
exercises the `0x0002` tag (#301) or a pad byte (#300).

## Decisions

- The cap for a soundbank Entry of the audio index rises from 16 MiB to 64 MiB, twelve times the
  largest live bank, so that a bank a few times larger than today's does not vanish from the index
  and report its events as `no_soundbank` (ADR-0018). `p4kx-extraction` already reads up to 512 MiB.
- The four quirks above degrade to `BankWarning`s of `Soundbank::warnings` instead of rejecting
  the bank, because other Wwise readers (vgmstream, wwiser) tolerate them as well. Structural
  damage still rejects the bank: a truncated section, a `DIDX` entry outside `DATA`, a `DIDX` of
  a size that is no multiple of 12, a repeated section, an `STID` that lists more names than it
  holds.
- `Wem::chunks` and `Wem::chunk` list every RIFF chunk, so `smpl`, `cue ` and `LIST` are
  available for the looping work (#95) wherever they lie.

## Cost of validating the whole Vorbis stream

`write_ogg` checks the first 16 audio packets with lewton (ADR-0011). Decoding the rebuilt Ogg to
its end, as a player would, costs more than the whole rebuild. On a stride sample of 6,161 Wwise
Vorbis WEMs of LIVE (596 MiB), CPU time summed over all threads:

| | Time |
| --- | --- |
| Rebuild with 16 checked packets, sample count and Ogg writing | 16.4 s |
| Decoding every rebuilt stream to its end | 111.2 s |

Every sampled stream decodes to its end (0 failures). Full-stream validation would make a rebuild
about eight times as expensive, for a failure that did not occur in the sample.
