# The vehicle implementation is patched by the Modification before it is read

Decided for issue #289.

A ship's `VehicleComponentParams` name an XML file, the vehicle implementation
(`Scripts/Entities/Vehicles/Implementations/Xml/<name>.xml`), that holds what the DataForge
Loadout does not: the parts of the hull with their mass and damage, the item ports with flags such
as `invisible`, treads and wheels. The Record's `modification` selects one `Modification` of that
file, and the file can name a patch file below `Modifications/`. `p4kx-assembly` reads the
implementation as an owned element tree (text XML or CryXml alike), applies the Modification to
the tree, and only then reads parts from it, so a Modification that sets `skipPart` or `flags`
takes effect before anything is interpreted.

- **Lookup.** The file is the Record's `vehicleDefinition`; if the Archive has none of that path,
  the file of the same name in the implementation folder, then the file named after the Entity's
  class name. Paths match ignoring case. The patch file is the Modification's `patchFile`, else
  `Modifications/<definition name>_<modification>.xml`, else `Modifications/<class name>.xml`.
- **Patch file.** The three reference tools disagree on what a patch file is, and the Archive's
  files are encrypted, so the rule is the union that none of them contradicts. A child of the
  patch root named `Vehicle`, and a child without an `id` such as `Parts`, replaces the child of
  the same name of the base wholesale (the parts-only files of the F7CM and the 350r; the full
  override of the Sentinel). A child with an `id` is merged into every base element with that
  `id`: its attributes are set and its children replace the element's. The base's own
  `Modifications` element is never replaced.
- **Elems.** After the patch, each `Elem` of the selected Modification sets the attribute `name`
  to `value` on every element whose `id` equals `idRef`, wherever it sits (`Part` or `ItemPort`).
  Attribute names and element names match ignoring ASCII case (`minSize`/`minsize`,
  `damageMax`/`damagemax`).
- **Parts.** Parts marked `skipPart` are dropped with their subtree after the Elems, so an Elem
  can bring one back. A part's port name is the part's `name`, matched ignoring ASCII case like
  every port key (ADR-0012); StarBreaker's case-sensitive matching is deliberately not followed.
  Numbers with stray characters are read leniently (`12.5kg` is 12.5; no digits is none).
- **Visibility.** A port of the Loadout tree is `invisible` when the part of that name carries
  the flag `invisible` on its item port, `visible` when it carries the part without it, and
  unspecified when the implementation has no such part. A Modification that clears the flag
  therefore shows in the tree.
- **Treads, wheels, landing gear.** Parts of class `Tread` list the wheel part names they run on,
  parts of class `SubPartWheel` are the wheels. Landing gear is not in the XML: the entity that
  holds it carries a `VehicleLandingGearSystem` component whose `gears` name a bone each, and
  that is listed on its node.
- **Failure.** A missing or corrupt implementation, patch file or Modification is a
  `vehicle_implementation` issue of the Entity; the rest of the tree still resolves, and a patch
  that cannot be read leaves the base implementation in place.
- **Limits.** Text XML is first scanned for its element depth and CryXml and text trees are read
  with a depth of at most 128 and at most 2^21 elements, so a hostile file cannot exhaust the
  stack (the XML parser recurses per element). The file size limit is that of XML Loadouts.

The library API is `VehicleImplementation::read` (an implementation without a Loadout tree, for
exports) and `EntityNode::vehicle`, `EntityNode::landing_gear` and `PortNode::visibility` (in the
tree). Mass, hull points and number noise beyond the lenient reading above belong to #316 and
#304.

## Considered Options

- Replace the inner XML of id-matched nodes only (scunpacked) or the whole node (ScDataDumper):
  each ignores the parts-only files Nova reads, so those ships would silently get the base tree;
  merging attributes and replacing children covers both.
- Treat every patch file as the variant's whole part tree (Nova): wrong for patch files that name
  single parts by `id`.
- Apply the Elems to the parsed parts instead of the tree: an Elem can target any attribute of
  any element, which the parsed model does not keep.
