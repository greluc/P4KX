# An Interior places the props and lights of its .soc entities

Decided for issue #137. Builds on ADR-0081. Amended by ADR-0199: props from class Records, tint
palettes, the material a prop names, lights of prop entities and the light frame of a node with
children; the stored light colour is linear, not sRGB. Evidence: LIVE socpak of the RSI Mantis body (21 props,
46 lights written, 6 planar lights reported, 1.3 s) and of the Idris front (1 prop, 12 planar
lights reported).

## Decision

The entity chunk of a `.soc` file (`CrCh` type `0x0004`, `CryXmlB`, root `Entities`) lists
`Entity` elements with `Name`, `Pos` (x,y,z), `Rotate` (quaternion w,x,y,z), `Scale`,
`EntityClass` and the components of the entity. `p4kx-model` reads it (`SocEntities::read`) into
props and lights, in the container's space like the `IncludedObjects`; the transform is
scale, then rotation, then position. Entities that are neither are only counted.

- **Props.** An entity whose `PropertiesDataCore/EntityGeometryResource` names a model (three
  nested `Geometry` elements, the last with `path`) is a prop. `Interior` places it as a part of
  kind `Prop` and resolves its model like a placed model: the container of the `.soc`, the
  containers around it, the Archive. Entities whose geometry comes from their class Record
  (`EntityClassGUID` only) are not placed: that needs the DataForge, which an Interior does not
  open yet.
- **Lights.** A `Light` entity (`PropertiesDataCore/EntityComponentLight`) and every `Light` of
  a `LightGroup` (`EntityComponentLightGroup/BakedInLights`, placed by the group's transform
  composed with the light's `RelativeXForm`, `rotation` w,x,y,z and `translation`) are lights.
  The light of each state (`defaultState`, `auxiliaryState`, `emergencyState`, `cinematicState`)
  is read; the values of the first state in that order whose intensity is above zero are used,
  else of the default state.
- **glTF.** Lights are written with `KHR_lights_punctual`; the scene node of a light holds it
  (`SceneNode::with_light`) and `GlbSummary::lights` counts them.

| CryEngine | glTF | Notes |
| --- | --- | --- |
| `Omni`, `SoftOmni` | `point` | `bulbRadius` and softness are not expressible |
| `Projector` | `spot`, outer angle `FOV / 2`, inner angle 0 | `FOV` 0 gives 45 degrees; `focusedBeam` and the gobo texture are not used |
| `Directional` | `directional` | |
| `Ambient`, `Planar`, anything else | not written | reported as `light_unsupported` with the type and count |

- **Direction.** A `CryEngine` light shines along its local +X axis (measured on the Mantis: with
  w,x,y,z and +X, half of the 30 projectors point down within 60 degrees, with any other
  quaternion order or axis at most 30 % do). glTF lights shine along -Z, so the writer turns the
  node: its transform composed with the rotation that takes +Y, which is glTF's -Z in `CryEngine`
  axes, to +X.
- **Colour.** The colour of a state is read as sRGB and written linear. With `useTemperature` it
  is multiplied by the colour of a black body at the state's `temperature` (Tanner Helland's fit,
  1000 to 40000 K, scaled to a largest channel of 1). Both are assumptions, not measured; the
  Basetool and viewers should treat the colours as approximate.
- **Intensity and range.** The intensity is the engine's value unchanged, not converted to
  candela or lux: the unit is not documented and a factor such as StarBreaker's x1500 is a
  heuristic. `lightRadius` is the range when it is above zero. `fadeParams`, shadows, flicker,
  styles and the other states are not written.
- **Writer.** Colours are clamped to 0 to 1, a negative or non-finite intensity is zero, a range
  that is not positive is left out, and spot angles are kept within 0 to a quarter turn with the
  inner angle below the outer one. Values that overflow single precision (`Pos`, `Scale`,
  `RelativeXForm`) make the entity unusable: it is left out and counted in `entities_left_out`.
  The extension is declared only when lights exist.
- **Reported.** `light_unsupported` (the light types above), `entities_left_out` (props or lights
  with a position, rotation, scale or light type not usable), plus the model issues of the props.
  Limits: 2^20 entities and 2^17 props and lights per `.soc` file, and the part limit of
  ADR-0081.

## Not done

- Props whose geometry comes from the class Record, and the tint palette of a prop
  (`IncludedObjects` palettes, the entity's `Material`): they need the DataForge.
- Light states beyond the one chosen, animation, and the gobo and projector textures.
- Records of kind 16 in the `IncludedObjects` are skipped (ADR-0193).
