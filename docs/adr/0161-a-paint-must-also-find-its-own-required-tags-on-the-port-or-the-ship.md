# A paint must also find its own required tags on the port or the ship

Decided for issue #434, a follow-up of #399 and PR #428. Amends the paint rule of ADR-0054's
catalog (`docs/basetool-catalog.md`, "Paints").

## Context

A paint fitted a ship when one paint port of the ship required only tags the paint carries. The
paint's own `AttachDef.RequiredTags` were never read, so a paint made for one ship fitted every
ship that shares the port.

## Decision

- A paint also fits a ship only when every tag of the paint's `RequiredTags` is offered by the
  same port: its `portTags`, its `requiredTags` (the Omega port requires `Paint_Omega` after the
  Modification while it lists `Paint_Mustang` as port tag), or the `AttachDef.Tags` of the ship's
  Record. A tag below an offered tag in the tag tree counts as that tag; names compare ignoring
  case. A paint without `RequiredTags` is unaffected.
- Measured on LIVE (4.7): 1,064 paints, 1,039 assigned, 25 unassigned before and after. The
  check removes 40 ship assignments of four paints and no paint loses its last ship. All 40 are
  false positives: the F7A Mk II desert paint on ten F7C variants, the C8X Pisces Expedition paint
  on eight C8 and C8R variants, and two Cutlass Black paints on the eleven Red, Blue and Steel
  variants each. Without `requiredTags` among the offered tags the Omega paint would lose the Omega.
- The shape of the catalog is unchanged, so `schemaVersion` stays 1 (ADR-0054). The selection of
  an existing kind does change, but only by removing assignments that were wrong; a reader that
  relied on them relied on a defect.

## Consequences

- The ships are grouped by their paint ports and their Record tags, so a ship with other tags is
  tested on its own.
- The Idris-M still fits the paints of the Idris-P: those paints require no tag of their own.
