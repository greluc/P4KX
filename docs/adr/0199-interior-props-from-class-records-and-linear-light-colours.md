# Interior props come from class Records and light colours are linear

Decided for issue #527, the follow-ups of ADR-0087. It amends ADR-0087 (props, tint palettes,
light colour, light frame) and ADR-0081 (palettes of the object table). Evidence: the `.soc` files
of every socpak of the LIVE Archive (684498 entities, 1.4 million stored colour channels) and the
Mantis body and Idris front containers.

## Decision

**Props from the class Record.** An entity that names the Record of its class
(`EntityClassGUID`) and no geometry of its own gets the geometry of that Record when the Interior
is built with the DataForge: `Interior::build_with` takes an `InteriorForge`
(`InteriorForge::new(&forge)`), as do `EntityInterior::build_with` and the CLI and GUI exports.
The Record's `SGeometryResourceParams` gives the model, the material library and the tint palette
of the main geometry; sub-geometries and material variants (tags) are not used. The entity becomes
a part of kind `Prop` like a prop with geometry of its own. An entity of a class without
geometry, or of a class the DataForge lacks, stays unplaced and uncounted; an unusable position is
counted in `entities_left_out`. Records are looked up once per GUID. Without a DataForge nothing
changes. The Idris front gains 89 props (142 to 231) and the Mantis body 3 (21 to 24); all lie
inside the container's bounds.

**Material library.** The library a prop names (`Material` of its geometry, or of its class
Record) replaces the one its model names, as in the Assembly (ADR-0040); each library is read
once.

**Tint palettes.** With a DataForge a part draws with, in this order: the palette its entity
chooses (`Palette` of its geometry, or of its class Record), the palette of its row in the object
table of the `.soc` file (the Record path is looked up in the DataForge), else the default
palette of its material library. A palette that cannot be used (no Record at the path, no GUID, a
child path, which is not supported) is a `palette` issue and the part falls back to the next
choice. Equal palettes are one object, so parts share meshes as in the Assembly. The Blender
package takes the palette of an Interior part like that of an Entity's part. 2111 of the 3163
parts of the Idris front have a palette.

**Lights on prop entities.** An entity with geometry of its own or a class Record keeps the
`BakedInLights` of its light group; the reader no longer stops after the geometry. The LIVE
Archive has no entity with geometry of its own and baked-in lights (0 of 40787), but 6267 entities
that name a class Record carry some (doors, airlocks, lamp fixtures), which are lights as before
and now also props when their class has geometry.

**LightBox.** The 19052 `LightBox` entities of the Archive carry only a box (`BBox_os`,
`FadeBox`): a volume that fades the baked lighting, with no colour, intensity or type. glTF has no
such light, so they are not written and not reported.

**Light frame.** The writer turns a scene node that holds a light by `LIGHT_FRAME` (the engine's
+X to glTF's -Z) only when the node has neither a model nor children. Otherwise the node keeps
its transform and the light sits on an extra child node of the same name with `LIGHT_FRAME` as its
transform, so a public `SceneNode` with a `part` or children is not rotated by a quarter turn.

**Light semantics.**

- *Colour space.* The stored colour is linear, not sRGB: of 1397641 channels strictly between 0
  and 1, 1390106 (99.5 %) are the linear value of an 8-bit sRGB colour (linear to sRGB times 255
  is an integer within 0.004), against 31777 (2.3 %) that are 8-bit values as they stand
  (`real_soc_lights`). ADR-0087 read them as sRGB and made them linear a second time, which drew
  the lights too dark; they are written as stored. The black body factor of `useTemperature` was
  already linear.
- *Intensity.* The unit cannot be derived from the files: the values of the default state have a
  median of 1 for `Omni`, 0.05 for `SoftOmni`, 1 for `Projector`, 0.5 for `Planar` and 0.8 for
  `Ambient`, a 90th percentile of 7 for `Omni` and 30 for `Projector`, and outliers up to 7.8e10.
  They stay the engine's raw value (ADR-0087).
- *`focusedBeam`.* Every light type carries it: set on 135765 of 207k `Omni` and 171584 of 264k
  `Projector` lights (about two thirds), whatever the opening angle. It cannot be a cone shape, so
  it stays unused.

**`entity glb --interior`.** Its JSON report has `interior.lights` and, with the Interior, the
nodes of kind `light`, since #138; `cli-json.md` describes both.

## Considered Options

- Open the DataForge inside `Interior::build`: every container of an Entity's Interior would read
  it again. The caller owns the DataForge and passes it.
- Place every entity with a class GUID: most classes (audio triggers, particle effects, areas,
  navigation) have no geometry and are found out only through the Record, which is what the
  lookup does.
- Reject a scene node with a light and children: the extra child node keeps every caller working.

## Not done

- Sub-geometries and material variants of a class Record (`SubGeometry`, `materialVariants`),
  which the Assembly applies by tags.
- Child palettes (`ChildPath`) and the tint palette of the object table's rows when no DataForge
  is given.
- `LightBox` volumes, light states beyond the one chosen, animation and the projector textures.
