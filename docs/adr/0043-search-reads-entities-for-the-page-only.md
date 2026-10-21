# A trigger search scans the index and reads the Entities for one page only

Decided for issue #144.

`p4kx wwise events` searches the names of audio triggers and of the Wwise events they post. The
audio index of ADR-0018 already holds every ATL trigger in the order of its definitions' Entry
paths, so a search is one pass over those triggers with the non-allocating `PathPattern` of the
Filters (substring or glob, ignoring ASCII case): no second index of names is built. The pass
counts every hit and keeps the hits of the requested page, at most 10 000, so the result size is
bounded and the same search gives the same pages. Events that no ATL definition names are not
found by name: a soundbank holds only their hashed IDs.

"Which Entities name this trigger?" cannot be answered from the audio index; the Record of every
Entity has to be read, as ADR-0029 says for References. `TriggerUsers` reads the Record tree of
every Entity once, in parallel with one `TreeBuilder` per rayon task, and keeps for the wanted
triggers only the indices of the Records that name them (`u32` per Entity and trigger, sorted, so
the order is that of the Records whatever the number of threads). The command passes the triggers
of the requested page as the wanted ones: the memory is bounded by the page, and the Records are
read once per command, not once per trigger. The Entities are counted per trigger and at most
`--entities` of them are listed. `--no-entities` skips the DataForge for a fast search. An Archive
whose DataForge cannot be read still lists the triggers, with a warning.

Triggers are taken from `GlobalResourceAudio.audioTrigger` as `p4kx entity audio` takes them; the
wider discovery of #278 is not part of this.

## Considered Options

- Index the Entities of all triggers once per Archive and cache them: a command that lists 100
  triggers would hold the users of 100 000 triggers; the cost of reading the Records is the same.
- Read the Records of an Entity only when it is listed: the count per trigger needs all of them.
- Match the search against a lower-cased copy of every name kept in the index: the extra memory
  buys nothing, because the pattern matching already ignores case without allocating.
