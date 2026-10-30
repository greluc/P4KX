# Glass alpha follows the glass colour

Decided for issue #855. Amends ADR-0100 (the glass row and the alpha rule) and keeps ADR-0253.

## Context

ADR-0100 maps glass to a Principled BSDF with Transmission Weight 1 and IOR 1.45, rendered with the
Blended render method and an alpha of 1 (the `opacity`). In EEVEE a blended surface cannot transmit
what is behind it: the Blender manual says the Blended method is incompatible with raytracing, and
a transmissive BSDF falls back to the light probes, so it shows the world colour. Glass was opaque
in EEVEE and in the viewport's Material Preview, and clear in Cycles. Measured in Blender 5.2.3 with
a red cube behind a tinted plane: Blended, Dithered and Dithered with the material's raytraced
transmission all draw the world colour in EEVEE (the scene's raytracing is off by default and is not
the add-on's to change); only a lower alpha shows the cube. Cycles draws the cube in every case.

## Decision

- Glass keeps Transmission Weight 1, IOR 1.45 and the Blended render method.
- The alpha is `1 - 0.85 * luminance(Base Color)`: white glass has alpha 0.15, a darker tint is more
  opaque, black is opaque. The Base Color is the one of ADR-0253, so the palette's `glass` colour
  and the vertex-colour tint both act; with vertex colours the luminance is taken per pixel in the
  node tree (RGB to BW, then maths nodes).
- The description's alpha (`opacity`, an opacity map) multiplies the result.
- The rule is structural: the shader family and the colour, never a material or ship name. The
  surface type (`glass_thin`, `glass_dense`) is not used.

## Consequences

- The cockpit and everything behind clear glass is visible in EEVEE and the Material Preview.
- In Cycles the extra alpha only mixes in a transparent shader; transmission still tints.
- Glass whose tint is dark is opaque by this rule in EEVEE even where the game's glass is
  translucent; a better match needs the game's own shader.
- Surfaces drawn after the glass by Blended per-object sorting can still show in front of it.
