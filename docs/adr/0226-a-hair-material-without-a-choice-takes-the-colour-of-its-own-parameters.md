# A hair material without a choice takes the colour of its own parameters

Decided for issue #759. It amends ADR-0090, which left the lashes' shader as it is.

## Context

The lashes of a head are `HairPBR` hair cards: their texture is white and holds only the opacity,
and the colour comes from the shader's parameters (`BaseMelanin`, `BaseMelaninRedness`,
`BaseTintColor`, `DyeColor`, `DyeAmount`). A Character makes colour choices for its hair, brows and
beard, but none for the lashes. Without a choice no `Look` was applied, so the material kept the
plain colour of its file, white, and the character package and `chf glb` drew white lashes.

## Decision

**A `HairPBR` material that no choice reaches is coloured from its own parameters**, by the same
rule as a chosen hair (ADR-0090): the natural colour of the melanin and the redness times
`BaseTintColor`, mixed with `DyeColor` by `DyeAmount` where the library's hair takes a dye. The
parameters are those of the material file, so a head whose lashes the game makes dark draws them
dark. `Look::natural` is that look; the assembly applies it to every job that has no `Look` of its
own, and it changes nothing in a library without a hair material.

The alpha stays as the writer had it: the hair cards are cut out at 0.5 by the opacity map.

## Consequences

Every `HairPBR` card without a choice, not only the lashes, is no longer white. A piece with a
choice is unchanged.
