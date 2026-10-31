# Glass alpha follows the glass colour (amended: the alpha rule is replaced by ADR-0282)

Decided for issue #855. Amends ADR-0100 (the glass row and the alpha rule) and keeps ADR-0253.
**Amended by ADR-0282:** glass no longer takes an alpha from its colour; the colour tints a
Transparent BSDF. The measurements below stay as the reason for the 0.15 share.

## Context

ADR-0100 maps glass to a Principled BSDF with Transmission Weight 1 and IOR 1.45, rendered with the
Blended render method and an alpha of 1 (the `opacity`). In EEVEE a blended surface cannot transmit
what is behind it: the Blender manual says the Blended method is incompatible with raytracing, and
a transmissive BSDF falls back to the light probes, so it shows the world colour. Glass was opaque
in EEVEE and in the viewport's Material Preview, and clear in Cycles.

Measured in Blender 5.2.3 headless with a red cube behind a tinted plane, no per-material setting
helped: Blended, Dithered, Dithered with the material's raytraced transmission, Dithered with scene
raytracing on, a slab thickness and IOR 1.0 all drew the world colour in EEVEE. Only a lower alpha
showed the cube. Cycles drew the cube in every case.

## Decision

- Glass keeps Transmission Weight 1, IOR 1.45 and the Blended render method.
- The alpha is `1 - 0.85 * luminance(Base Color)`: white glass has alpha 0.15, a darker tint is more
  opaque, black is opaque. The Base Color is the one of ADR-0253, so the palette's `glass` colour
  and the vertex-colour tint both act; with vertex colours the luminance is taken per pixel in the
  node tree (RGB to BW, then maths nodes).
- The palette's `glass` colour is faded towards white by the shader parameter `GlassTintStrength`
  (default 1, clamped to 0..1), so a material that names the palette colour with a weaker strength
  is lighter and more transparent.
- The description's alpha (`opacity`, an opacity map) multiplies the result.
- The rule is structural: the shader family, the colour and the parameter, never a material or ship
  name. The surface type (`glass_thin`, `glass_dense`) is not used: the colour already separates
  clear from tinted glass.

## Consequences

- The cockpit and everything behind clear glass is visible in EEVEE and the Material Preview.
- The alpha also applies in Cycles, where the Principled BSDF mixes in a transparent shader by it.
  Cycles glass is therefore lighter, less tinted and casts lighter shadows than before; on the
  Cutlass Black canopy the interior is more visible.
- Dark tints are nearly opaque in EEVEE even where the game's glass is translucent (#905).
- Surfaces drawn after the glass by Blended per-object sorting can still show in front of it.
