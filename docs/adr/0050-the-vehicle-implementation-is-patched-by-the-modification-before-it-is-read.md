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
  Numbers are read as below.
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
- **Limits.** Text XML is first scanned for its element depth (comments, CDATA and processing
  instructions are skipped from the end of their opening, as the parser does), and trees are read
  with a depth of at most 128, so a hostile file cannot exhaust the stack (the XML parser recurses
  per element). One budget covers reading, patching and the Elems: at most 2^21 elements, 2^22
  attributes (an attribute set costs the attributes it scans) and copied bytes at four times the
  input read. It stops CryXml nodes that share one attribute range, a patch copied into many
  elements with one `id`, and an Elem fanned out over them. A patch that exhausts it, or nests
  the tree deeper than 128, is an issue and the unpatched base stays. The file size limit is that
  of XML Loadouts.
- **Visible outcomes.** A patch file that dangles (even when a fallback file is applied), a
  replacing patch child with no base counterpart and a patch `id` that matches nothing are
  `vehicle_implementation` issues, and `VehicleImplementation::modification_report` counts the
  patch shapes and the unmatched `id`s and `idRef`s, so a real Archive can show whether the rule
  above fits.
- **Paths.** Every path read lies below `Scripts/Entities/Vehicles/Implementations/Xml/` (an
  optional leading `Data` is dropped); a `.` or `..` segment, a drive colon, or a separator in a
  class or Modification name makes the candidate unusable.
- **Numbers.** A number is the longest numeric prefix of the trimmed value (`12.5kg` is 12.5,
  `1,5` is 1, `v2 1500` is none); `VehiclePart::attribute` gives the raw value for readers with
  their own rule (#304, #316).
- **Layers.** `p4kx-assembly` is a service, and a service may depend on another service
  (ADR-0003), so `p4kx-export` can call `VehicleImplementation::read` directly.

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
