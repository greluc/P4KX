# Glass tints the scene behind it through a transparent shader in the add-on

Decided for issue #905. Amends ADR-0258 (the alpha rule) and ADR-0100 (the glass row); keeps
ADR-0253.

## Context

ADR-0258 gave glass the alpha `1 - 0.85 * luminance(Base Color)`. A dark tint is then nearly
opaque in EEVEE: the Cutlass Black's `glass_mat` (grey 0.16, alpha 0.86) and
`plastic_dirty_translucent` hid what is behind them, and in Cycles the same alpha lightened the
tint.

The game's `GlassPBR` descriptions on LIVE carry no transmission or opacity parameter. The glass
parameters are the colour (`diffuse`, or the palette's `glass` entry with `GlassTintStrength`),
`Thickness` (0.005 to 0.02 m, a slab depth with no reference length), refraction, glare, dirt and
wear values; `opacity` is 1 on every glass material of the Cutlass Black. The colour is the only
parameter that says how much light passes, and it acts as a multiplier on what is behind the glass.

Measured in Blender 5.2.3 headless, with a red cube behind a plane: in EEVEE a blended Transparent
BSDF multiplies the backdrop by its colour, matching Cycles, where a Transparent BSDF and a
Transmission BSDF both do. A blended Principled BSDF with alpha 1 draws the world colour instead.

## Decision

- Glass is a Mix Shader: `GLASS_BSDF_SHARE` (0.15) of the Principled BSDF (Transmission Weight 1,
  IOR 1.45, as before) and the rest a Transparent BSDF whose colour is the glass Base Color of
  ADR-0253. The surface stays blended. The Principled BSDF keeps its Alpha at 1.
- The share is a constant, not a function of the colour: the tint decides how dark the scene
  behind is, never whether it is visible. 0.15 is the value ADR-0258 measured for clear glass, so
  clear glass looks as before.
- The description's alpha (`opacity`, an opacity map) fades the whole surface to a white
  Transparent BSDF with a second Mix Shader; without an alpha there is no second mix.
- `Thickness` is not used: no reference length is known that would turn it into an absorption.
- The rule is structural: the shader family, the colour and the parameter, never a material or
  ship name.

## Consequences

- Dark glass dims the scene behind it by its colour in EEVEE and the Material Preview, and EEVEE
  and Cycles agree within a few per cent on the Cutlass Black's `glass_mat` and
  `plastic_dirty_translucent`.
- Cycles glass is no longer lightened by an alpha; it shows the colour as the game does.
- Surfaces drawn after the glass by Blended per-object sorting can still show in front of it.
- Removing the vertex colours from glass on a mesh without colours (ADR-0253) sets the colour of
  the Transparent BSDF and of the Principled BSDF to the plain colour.
