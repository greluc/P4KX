# Record files by record path, with the Record name for shared paths

A Record export writes one file per Record "under its record path", but in the live DataForge
55,460 of 117,709 Records share a record path with others (487 paths; one tag database holds
18,892), and even record path and name together are not unique. A Record alone at its record path
is written to `<record path>.xml`; Records that share a record path go into the folder
`<record path>/`, each to `<Record name>.xml`; when such a file collides with another ignoring
case, the GUID is added (`<Record name>.<GUID>.xml`), and a name that is no plain file name is
replaced by the GUID (`<GUID>.xml`). The file a Record gets depends on the whole DataForge, never
on the selection, and a folder takes the spelling of the first Record in definition order that
needs it. Decided by the owner.

## Considered Options

- `<record path>.<GUID>.xml` for every Record of a shared path: unique, but GUIDs make poor names.
- One file per record path holding all its Records under a wrapper element: mirrors the game's
  source files, but gives a second document shape and breaks "one file per Record".
