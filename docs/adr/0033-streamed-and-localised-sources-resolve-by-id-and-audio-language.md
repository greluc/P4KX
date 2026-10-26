# Streamed and localised sources resolve by ID and Audio language

Decided for issue #143. Evidence: the LIVE Archive, checked with the real-Archive tests of
`p4kx-assembly` (`real_streamed_localised`).

## Streamed sources are Loose WEMs found by ID

A Sound object whose storage is streamed or prefetched names its media by the WEM ID. The Archive
holds those WEMs as separate Entries: `Data\Sounds\wwise\Media\<id>.wem` (76,751 in LIVE),
`Data\Sounds\wwise\Media\<language>\<id>.wem` (40,316, all `English(US)`), and the layouts
`wwise\<id>.wem` and `wwise\<language>\<id>.wem` that other builds and tools use. The audio index
reads every `.wem` Entry below `Data\Sounds\wwise\` once, ignoring ASCII case and the separator
kind (ADR-0023), keeps the ones whose file name is a decimal `u32`, drops a `Media` folder from the
path, and takes the one folder left as the Audio language. A path with more folders, or a name
that is no number, is not a Loose WEM. The lookup is a hash of the ID, never a path composed from
an ID or a name a soundbank states, so no soundbank can steer a read or a write to another place.
The count is capped (`MAX_LOOSE_WEMS`); the first Entry beyond the cap is reported as an issue.

The index keeps its own folded paths instead of the `PathIndex` of `p4kx-extraction`: a service
crate may not depend on another service crate (ADR-0003), and the lookup by ID needs the parsed
name, not a path.

A prefetched source resolves to its Loose WEM, which holds the whole sound; only when the Archive
has no such Entry does the location stay the embedded start.

An embedded source that no soundbank embeds is looked up as a Loose WEM as well, and in LIVE this
is the main path, not an exception. ADR-0021 recorded that 99 % of the Sound objects with in-memory
media have it in no soundbank's `DIDX` and deferred locating them to #143. Over one third of all
triggers of LIVE the embedded sounds resolve as follows: 84,863 to a plain Loose WEM, 2,646 to a
Loose WEM in a language folder, 645 to a WEM held in a soundbank, 8 not located.

## The language flag

The source bits after the in-memory size of a Sound object carry the language flag in bit 0
(`SoundSource::source_bits`, `is_language_specific`). Check: of 8,187 localised sounds over a fifth
of all triggers of LIVE, the 7,822 with the flag all lie in the `English(US)` folder, and none
lies elsewhere. A sound is localised when the flag is set or its soundbank lies in a language
folder.

## The Audio language

The Audio languages are the folders found below `Data\Sounds\wwise\` that hold a soundbank or a
Loose WEM, in name order ignoring ASCII case, spelled as the first Entry spells them. Nothing is
hard-coded except the *default* language, `English(US)`, which only serves as the fallback of the
command line and falls back itself to the first language the Archive has.

A `LanguageChoice` names the language asked for and the fallback:

- A localised event is followed in the soundbanks of the asked language, or of the fallback where
  the asked one has none; soundbanks that are not localised are always followed. An event whose
  localised soundbanks are all of other languages gets a `no_localisation` break and no variant.
- A localised Loose WEM is looked up in the asked language, then in the fallback, then, as a last
  resort, as a WEM that is not localised (shared voice WEMs, e.g. comm beeps, lie only in
  `Media`). Without a WEM a streamed localised sound keeps its variant, unlocated,
  with a `no_localisation` break.
- A variant states whether it is localised, the language it was resolved in and whether that is
  the fallback.
- A choice without languages (`LanguageChoice::any`, what `resolve_trigger` uses) follows every
  language of the soundbank an event starts in and takes a Loose WEM from the first language in
  path order. A reference that leaves that soundbank follows the first language only (ADR-0158).

On the command line `--lang` must be one of the Audio languages (exit code 2 otherwise, listing
them) and `--fallback-lang` defaults to the default language. Without `--lang` only the fallback
is resolved, so a command line never answers with every language at once.

## `entity audio --sounds`

`p4kx entity audio` stays a listing of triggers, events and soundbanks that reads each soundbank
once. `--sounds` resolves every trigger as `wwise resolve` does, which reads the soundbanks of
each trigger again, and lists the variants with source, language and location under it. It is
opt-in because an Entity names a hundred triggers and the soundbanks reach 32 MiB.

## Considered Options

- Compose the Loose WEM path from the language and the ID and look it up with the `PathIndex`:
  needs the layout fixed in code and a dependency from a service crate on another.
- Treat a language-specific source as the language of its soundbank only: misses the shared WEMs
  and the language-specific sounds of soundbanks that are not localised.
- Follow every language of a localised event and list all: a multiple of the variants for one
  voice line, and no answer to "which one plays".
