# Nested prefabs are resolved by GUID with composed transforms

Decided for issue #310. Evidence: all 509 files below `Data\Prefabs` of LIVE (14939 prefabs),
read and resolved by the `real_prefabs` tests of `p4kx-assembly`. scdatatools (MIT,
`engine/prefabs/__init__.py`) reads prefab libraries too; its child placement adds the child's
position to the parent's without turning it by the parent's rotation (`prefabs/__init__.py:123-124`).

## Decision

- **A prefab library is an XML file below `Data\Prefabs`, text or `CryXml`** (`p4kx_model::PrefabLibrary`).
  Its root is `PrefabsLibrary` or, in some files, an element named after the library
  (`ht_d_light_fixtures`); what makes it a library is the `Prefab` children. The `Name` of the
  root is not the file's path (183 prefabs name another `Library` than their file), so a library
  is addressed by its file: the path below `Data\Prefabs` without `.xml`, ignoring case.
- **Objects are flattened.** A `Group` holds its members in a nested `Objects` element, their
  `Pos`, `Rotate` and `Scale` relative to the group; the group is left out and its transform is
  applied to the members. Groups nest at most 64 deep, further members are counted as left out.
  An object with a `Parent` other than its group, or an `AttachmentTarget`, is flagged `attached`:
  its transform is relative to that attachment and is used as it stands.
- **Transforms are composed as matrices** (`Transform::then`): the child's position is turned
  and scaled by the parent's rotation and scale before the parent's position is added. This is
  the case scdatatools gets wrong.
- **A nested prefab is found by its `PrefabGUID` and, first, its `PrefabName`.** `PrefabName` is
  the library name and the prefab name joined by `.`, and both may hold dots, so every dot is
  tried as the split. When the prefab of that name has the object's GUID, or the object names no
  GUID, it is used. Otherwise the GUID is looked up in an index of every library, built once by
  reading them four at a time (2.4 s for LIVE, 509 files, so memory is at most four libraries and
  their trees) and dropped again except for the
  libraries already loaded. Without the GUID lookup 6416 of the references in LIVE found no
  prefab; with it 1235 remain, which no library holds under name or GUID.
  An `Id` held by several libraries (102 in LIVE) resolves to the library of the name when it
  holds it, else to the first in sorted path order.
  When the name finds a prefab of another `Id` and no library holds the GUID,
  the prefab of the name is used and an issue says so (10 in LIVE).
- **Resolution is bounded.** A prefab that is being expanded further up the same chain is not
  entered again (`cycle`; one in LIVE). The chain is at most 16 prefabs long (`too_deep`), the
  result holds at most 524288 objects (`too_many_objects`) and the resolution looks at most at
  4194304 objects (`too_many_visits`), so a fan-out of prefabs that nest each other many times
  ends. A prefab that is not expanded stays in the list as the object that names it, with an
  issue (`missing`, `cycle`, `too_deep`). A library of more than 64 MiB is refused. At most 4096 issues are listed, the last says the rest is left out (`too_many_issues`); a `guid_mismatch` is listed once per prefab and GUID. Libraries are
  untrusted: no offset or length is trusted, nothing panics, every failure is a typed error.
- **The resolved objects share the library they come from** (`Arc`), so a resolved prefab of
  hundreds of thousands of objects copies no name, path or class.
- **`p4kx prefab list`** lists the libraries or the prefabs of one, **`p4kx prefab dump`** prints
  a resolved prefab as a table or, with `--json`, in the shape of `docs/cli-json.md`. Issues
  leave the exit code 0; a library or prefab the Archive lacks is 2, one that cannot be read 3.

## Not decided here

- How a socpak Interior refers to the prefab it was built from: compiled socpaks hold no prefab
  names, the prefabs are baked into their `.soc` files. The real-Archive test shows that the
  `crg_access` prefab of `MISC_Starfarer_Int` names models the Starfarer cargo deck places. The real-Archive test takes `crg_access` and `gemi_int_bdall_main` and requires five or more models placed once in both, with the same pairwise distances within 0.05 m.
- Objects whose geometry comes from their class Record (most `Entity` objects) have no `model`;
  reading it needs the DataForge and stays with the Entity code.

## Survey

LIVE: 509 files under `Data\Prefabs`, 14939 prefabs, 14825 distinct `Id`s (102 in several
libraries; the one in the library of the name is preferred). All prefabs resolve in 0.1 s once
read, 1.16 million objects in total, the slowest under 5 ms. One file (`Categories\Global`) is no
library.
