# Field statistics are counted per path in one parallel pass over the instances

Decided for issue #267.

A Query needs the right field, and DataForge has tens of thousands of field paths, many of them
only below one polymorphic pointer. `FieldStatistics` in `p4kx-dataforge` answers "which paths
exist, how often, of which kind, between which numbers" for all Records or for a selection.

The pass reads the instance bytes of each Record through the cached struct layouts, as the survey
and the Record tree do, and builds no Record tree: a tree allocates every value, and the statistics
need only one counter update per value. Each rayon task keeps a trie of path segments: a node is
a field name, or the name of the actual struct a strong pointer owns, which is how a path tells
`SWeaponActionFireRapidParams/fireRate` from `SWeaponActionFireSingleParams/fireRate`. Array
positions are not part of a path. The child of a node is found once per struct and node and cached
by field position, so a value costs an index and an increment; only the first value at a path
hashes a name.

Every node holds compact accumulators: counts of values, Records (a Record counts once, tracked
with the last Record seen), empty values and values per kind; the smallest and the largest number;
and the 64-bit fingerprints of the first 32 distinct values, after which only a flag is kept.
All of them are merged by addition, union and minimum or maximum, so the result does not depend on
how rayon splits the Records or in which order it merges the tries: the output is sorted by path
and is the same for any number of threads. Strings are fingerprinted from the text table without
copying them.

Memory is bounded three ways. A trie holds at most 1,048,576 paths; more is an error rather than a
silently incomplete result, and since every partial trie is a part of the union, whether it is hit
does not depend on the split. A Record is read at most as far as `MAX_RECORD_VALUES` values and
`MAX_NESTING_DEPTH` levels, like a Record tree, and an instance owned twice is read once. LIVE has
about 87,500 paths over 118,000 Records; counting them takes about two seconds with the Archive
opening included; the amendment below gives the memory.

Empty values are values: a null pointer or Reference, an empty text or a null GUID is counted in
`values` and `empty` and never in `distinct`, `min` or `max`, so absence does not read as zero.
`-0.0` and `0.0` are normalised to one value before they are compared or fingerprinted, and NaN
is no number. An empty array has no element and adds nothing.

The same walk serves the completion of #117: `FieldStatistics::get` and `children` give the kinds
at a path and the paths one segment below it, so a completion that wants the paths that really
occur in the data needs no second walker. The completion from the schema stays as it is; it knows
the fields no Record has a value for.

## Amendment: bounded parts, shared layouts and all-or-nothing Records (issue #389)

- The Records are folded in at most as many parts as rayon has threads (`with_min_len` of the
  Records divided by the threads), and each part keeps one trie. The memory above the DataForge is
  therefore at most `threads` tries, each at most 1,048,576 paths, and no longer one per task rayon
  happens to split off.
- The struct layouts are read once into a cache that all parts share (`SharedLayouts`, bounded by
  `CACHE_FIELDS` fields in total; further layouts are read again at each use), instead of one cache
  per part.
- A Record is walked into a list of pending values and counted only when the walk ends: a Record
  with more than `MAX_RECORD_VALUES` values adds nothing, not even the values found before the
  limit, which is what `docs/query.md` says.
- The pending values of the Record being walked take about 64 bytes each, up to
  `MAX_RECORD_VALUES` (2^20) values per part, so up to 64 MiB per part and 64 MiB times the threads
  in all (2 GiB at 32 threads) for a crafted Record at the limit; a real Record has far fewer, and
  a buffer above 2^16 values is released after the Record. This is a bound on top of the tries, not
  a growth with the number of Records.
- The paths a dropped Record made stay in the trie: a Record with too many values adds no counts,
  but the paths it walked before the limit still count towards `MAX_FIELD_PATHS` and can raise
  `TooManyPaths`. The walk of a Record is the same on any split, so the outcome does not depend on
  the number of threads.
- The values of a Reference are the GUIDs it links to and are fingerprinted like any value;
  strong and weak pointers and inline structs have no value, so their `distinct` is 0.
- On LIVE (117,709 Records, 87,514 paths, 32 threads) the working set during the count grows by
  about 53 MB above the memory of the opened DataForge, where it grew by about 331 MB with one
  trie and one layout cache per task. The bound is the number of threads times the paths each
  trie holds, so a crafted DataForge near the path cap on many cores is bounded by the same rule
  and no longer by the number of tasks.
- The fingerprint is 64 bits, so two different values collide with a chance of about 2^-64 per
  pair; numbers are fingerprinted exactly.

## Considered Options

- Build the Record tree of every Record and walk it: the tree allocates for every value, and
  these counters need none.
- Keep every distinct value with its count (top values): memory grows with the data, and the
  question this answers is which field to use, not which values it has; 32 fingerprints say
  whether a field is constant, a flag or free, and the extremes say its range.
- Count by the declared struct instead of the actual struct of a strong pointer: paths of
  polymorphic arrays would merge fields of different structs, which is what the statistics are
  there to tell apart.
- Cap the paths silently and report a count: the cut paths would depend on the order the Records
  are read in, and the result would differ between runs.
