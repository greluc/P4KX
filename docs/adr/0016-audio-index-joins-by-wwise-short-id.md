# The audio index joins ATL triggers and soundbank events by Wwise short ID

An Entity's Record names audio triggers through `GlobalResourceAudio.audioTrigger`; the ATL
definitions below `Data\Libs\GameAudio\` map each trigger to Wwise event names; soundbanks know
their events only by ID. The resolution lives in `p4kx-assembly` (`audio`), so `p4kx-audio` stays
a format crate. The audio index reads every ATL definition and every `.bnk` Entry once per open
Archive, in parallel, and keeps only names and IDs: triggers by lower-case name, and for every
event ID the soundbanks whose `HIRC` section holds an Event object with it, both in hash maps.
An event name is matched to an ID by Wwise's short ID, the 32-bit FNV-1 hash of the name in ASCII
lower case; in the live data more than 95 % of the ships' triggers reach a soundbank this way.
When two definitions name the same trigger, the first by Entry path wins. The trigger references
of an Entity are read from its Record when it is asked for, not indexed for every Entity. Entries
that cannot be read are kept as issues; the rest of the index still builds.

## Considered Options

- Follow the `ATLPreload` of a trigger to the soundbank files its preload request names: cheaper,
  but a trigger's event often lives in a shared soundbank the preload does not name, and the
  preload says nothing about whether the event is really there.
- Index the trigger references of every Entity with the rest: makes reverse lookups possible, but
  reads every Entity's Record tree up front, seconds of work that listing one Entity never needs.
