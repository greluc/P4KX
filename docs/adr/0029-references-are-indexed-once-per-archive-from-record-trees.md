# References are indexed once per Archive from the Record trees

"Which Records refer to this Record?" cannot be answered from the stored data without reading
every Record: a Reference is stored in the instance of the Record that holds it, and nothing
points back. StarBreaker answers it per call by parsing the DataForge again and scanning only the
main Records, so it misses References held by non-main Records. The index must cover every
Record, keep the field and the array elements of each Reference, keep References to a GUID no
Record has, and be built once per open Archive: the DataForge of LIVE holds millions of
References.

`ReferenceIndex` in `p4kx-dataforge` reads the canonical tree of every Record
(`TreeBuilder::build`, one builder per rayon task) and walks it, so the field path and the array
indices of a Reference are exactly those of the Record preview and the Query, and a Reference
held by a struct nested in an array is found like any other. A Reference field, a weak pointer to
another Record and a dangling Reference are References; a Reference or weak pointer into the
Record that holds it is not, because "the Record refers to itself" says nothing about its place
among the other Records.

The References live in columns of `u32` without a heap object per Reference: source Record,
target Record (or none), an interned field shape with `[]` for every array on the way, and the
index of the element in each array in one shared buffer. A shape is `Components[].fireActions[].ammo`;
the text with indices, `Components[3].fireActions[0].ammo`, is built on demand. Each parallel task
collects its Records in order with a local interner; the tasks are joined in Record order, so the
outgoing References of a Record are one run, and incoming ones are grouped by target with one
counting pass. The work per Record is bounded by `MAX_RECORD_VALUES`, the index by
`MAX_INDEXED_REFERENCES` (2^26); a Record whose tree cannot be built adds nothing and is counted,
and an index that stopped at the limit says so. All of it runs outside any `OnceLock`
initialiser, because the build uses rayon.

The window builds the index lazily, the first time the "Referenced by" tab is shown, on a thread
of its own, and keeps it in the shared cache of the Archive next to the completion schema: the
build is serialised by a mutex, the result is stored afterwards, and a change of the Archive on
disk drops it with the DataForge. The CLI builds it for the one command. A Query with
`referencing` takes the index of the window's cache; elsewhere `Query::compile` builds one when
the Query needs it.

The file graph groups Records by record path (ignoring case, separators and `.xml`, as the
Record search does) and has an edge from file A to file B, A different from B, when a Record of A
refers to a Record of B. A chain of References through Records of one file ends at the files it
reaches, so the edges of the Records of a file already contain what a walk through the file would
add. Unresolved References give no edge. On LIVE this gives about 475,000 edges for 62,700 files.

## Considered Options

- Scan the instance bytes by struct layout instead of Record trees: fewer allocations per Record,
  but it repeats the ownership and weak-pointer logic of the tree builder and would give
  different field paths in corner cases.
- A path string per Reference: simpler, but 30 to 60 bytes more for each of millions of
  References.
- Build the index in the DataForge on first use behind a `OnceLock`: a rayon pool call inside the
  initialiser deadlocks when another thread of the pool waits for the same lock.
