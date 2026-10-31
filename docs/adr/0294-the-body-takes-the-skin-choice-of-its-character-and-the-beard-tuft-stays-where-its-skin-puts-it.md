# The body takes the skin choice of its Character, and the beard tuft stays where its skin puts it

Decided for issue #760. Evidence: the LIVE Archive, `masculinedefault.chf`, `femininedefault.chf`
and the 41 CHF files it ships.

## What the data says

**Skin tone.** A CHF holds a material choice for `body_itemport` beside the one for
`head_itemport`: a base material and, per sub-material (`body_m`, `limbs_m`), the same `BodyColor`
as the head. The Assembly applied only the head-port choices, so the default body kept its own
library (`m_body.mtl`, diffuse `m_body_02`, `SourceAverageColor` 0.41, 0.15, 0.11) and showed
pink next to a head rebased to the dark `BodyColor`.

**Beard.** The strands below the chin of `m_facial_hair_045` are the 87 vertices that the skin
weights mostly to `HH_short_beard_x00_y02`, a bone of the beard's own chain that no body skeleton
has (ADR-0079 maps it to its nearest parent). Its bind position lies 2 cm below the chin tip. The
raw skin already has them 1.4 cm (median) from the head surface, before any shaping. Across all
shipped beards the DNA following adds at most 6 mm to a strand's distance from the head, so the
following kernel does not cause it.

## Decision

- **The choice for `body_itemport` styles the default body** like a head item: its base material
  replaces the body's library and its colours rebase the skin sub-materials, see `Look`. It is
  carried by `Piece::body()`, a piece that makes no part, which `Head::dressed` appends when the
  Character made the choice. A Character without one keeps the default body.
- **The beard tuft is a data property.** The strands hang from a simulated chain that rests
  there in the game; the export has no simulation and writes the bind pose. They are not pulled
  towards the chin.

## Consequences

Body and head share the tone the Character chose. The tuft looks detached in a viewer that
draws the cards as opaque quads. Binding hair and beard to the facial joints (#726) is separate.
