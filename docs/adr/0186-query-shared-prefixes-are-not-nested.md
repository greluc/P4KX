# Query shared prefixes are not nested

The shared-prefix grouping of a Query (`query/shared.rs`) gives each column the longest steps it
shares with any other column, and each group records its prefix once per row. When columns split
into several groups over a common shorter prefix, that shorter prefix is walked once per group.
Recordings that replay their parent group's recording would walk it once in total. They are not
built.

- **Measurement:** the ship Query of `real_query_prefixes` (1104 Records, eight paths under two
  components, the benchmark shape that forms a group at one step and another at two) on the LIVE
  Archive of build 4.10.2, release, two rounds of five, median of 300 row runs, against the
  grouping on main before PR #691 (one group at the common prefix):

  | Grouping | Round A, shared (alone) | Round B, shared (alone) |
  | --- | --- | --- |
  | before #691 | 0.75 to 0.80 ms (0.87 to 0.92 ms) | 1.00 to 1.07 ms (1.15 to 1.28 ms) |
  | PR #691 | 0.77 to 0.81 ms (0.90 to 0.92 ms) | 0.81 to 1.03 ms (0.94 to 1.16 ms) |

  The ratio of shared to alone is 0.84 to 0.89 for the old grouping and 0.84 to 0.93 for the new
  one. The differences between the two builds are smaller than the difference between two rounds
  of the same build, because the machine is shared. On all Entities (29693 Records, six paths
  under one component) shared and alone are equal in both builds.
- **Why it does not pay:** the repeated walk is the array fan-out over the components of one
  Record, a few elements. Nesting would replace it by a replay of the parent's events into the
  child, and the measurement shows no room for a gain above the noise.
- **Bound:** at most half as many groups as columns, so the worst case is the same as walking
  every column alone. `CompiledQuery::shared_prefixes()` keeps returning flat groups.

## Considered Options

- Nested recordings (each group replays its parent's recording): byte-identical results are
  possible, but no gain is measurable on the shapes that exist, and the replay chain makes
  `Scratch` and the row loop harder to follow.
- One group at the shortest common prefix, as before PR #691: it walks the shared steps once but
  makes a column that leaves early shorten the prefix of the others, which PR #691 fixed.
