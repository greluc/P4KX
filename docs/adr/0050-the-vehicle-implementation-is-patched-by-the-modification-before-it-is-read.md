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
  `id`: its attributes are set and its children replace the element's, each new child taking over the
  attributes of the old child it pairs with (same element name and `name`) that it does not state
  itself, so an `id` that Elems address survives; a child whose `id` no base
  element has is a wrapper (the Harbinger's `<Parts id="modParts">`): if the base has a child of
  its name, the wrapper's descendants are merged into it by their own `id`s (the topmost with an `id`, at any depth), and only when none of
  them matches does the wrapper replace that child wholesale, so a mistyped wrapper on a partial
  patch does not drop the other parts; a wrapper with no base child of its name is an unmatched
  `id`. The base's own `Modifications` element is never replaced.
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
  parts of class `SubPartWheel` are the wheels. Landing gear is not in the XML: the Entity's
  `VehicleComponentParams.landingSystem` refers to a Record of type `VehicleLandingGearSystem`
  whose `gears` name a bone, a geometry path and a mass each, and that is listed on its node.
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
  of XML Loadouts. Each element also carries a vector of origins (`ValueOrigin`, ADR-0061), one
  entry per attribute that a patch or an Elem set, with a copy of the attribute name; the vector
  is cloned with the element, is bounded by the attribute count, and its name copies are not
  counted in the copied bytes.
- **Visible outcomes.** A patch file that dangles (even when a fallback file is applied), a
  replacing patch child with no base counterpart, and an `id` (of an element or of a wrapper's
  child) that matches nothing in the base and has no base child of its name to replace are
  `vehicle_implementation` issues; a wrapper that replaced a child wholesale is listed in the report
  (`replaced_ids`) without an issue, and `VehicleImplementation::modification_report` counts the
  patch shapes and the unmatched `id`s and `idRef`s, so a real Archive can show whether the rule
  above fits.
- **Paths.** Every path read lies below `Scripts/Entities/Vehicles/Implementations/Xml/` (an
  optional leading `Data` is dropped); a `.` or `..` segment, a drive colon, or a separator in a
  class or Modification name makes the candidate unusable.
- **Numbers.** A number is the longest numeric prefix of the trimmed value (`12.5kg` is 12.5,
  `1,5` is 1, `v2 1500` is none); `VehiclePart::attribute` gives the raw value for readers with
  their own rule (#304, #316). A remainder other than white space, `d`, `f` or `kg` makes the
  read partial, which the sums report (ADR-0128).
- **Layers.** `p4kx-assembly` is a service, and a service may depend on another service
  (ADR-0003), so `p4kx-export` can call `VehicleImplementation::read` directly.

The library API is `VehicleImplementation::read` (an implementation without a Loadout tree, for
exports) and `EntityNode::vehicle`, `EntityNode::landing_gear` and `PortNode::visibility` (in the
tree). Mass, hull points and number noise beyond the lenient reading above belong to #316 and
#304.

## Check on the live Archive

LIVE (1140 vehicles, 97 with a patch file) read with no unexpected issue. Patch files of all three
shapes occur: `Vehicle` children (Sentinel), id-less children (Taurus), a wrapper whose id matches no element but whose descendants' ids do (Harbinger) and
id children (350r, F7CM, Freelancer). The only issues are the wrecked Starfarer's Modifications,
which name patch files `MISC_Starfarer_Dead*.xml` that the Archive does not hold. 128 `Elem`s
match no element; they are not issues, and they are CIG data, not rule gaps: the `idRef` is the
`id` of a part that the same Modification's patch replaced wholesale (`modShield`,
`modFlightController`, `modWeapon00x`, `modpowerplant001`, `modName`), or no element of any
implementation has that `id` (`modMannequin`, `modCoPilotSeat`, `modHoloGeometry_*`,
`modLeftLegPedal`, `xmlMannequin`, `modDriverMissileControl`). No case difference is
involved.

The Harbinger shows why wrappers merge by descendant ids. Its patch holds a complete tree of 136
parts against 149 in the base; 133 of them lie below one of nine elements whose `id` the base has
(`modNose`, `modBody`, the wings and so on), and the other three are the root part and two
hardpoints directly below it. The root part has no `id` and a `mass` about 18.6 times the base
root's. The Harbinger's own `Modification` in the base sets the hull mass with an `Elem` on the
root part (`modName`), to a value within a few per cent of the base's, and re-tags the flight
controller (`modFlightController`). Replacing the tree wholesale lost both targets and took the
patch root's mass; merging by descendant `id` keeps them and leaves the hull mass to the
`Elem`. The patch's copy of the flight controller's item port states neither the `id` nor the
tags of the base's, which the Elem re-tags; taking over the partner's attributes keeps the `id`, so
the Elem finds it, instead of leaving a port with no tags at all.

## Considered Options

- Replace the inner XML of id-matched nodes only (scunpacked) or the whole node (ScDataDumper):
  each ignores the parts-only files Nova reads, so those ships would silently get the base tree;
  merging attributes and replacing children covers both.
- Treat every patch file as the variant's whole part tree (Nova): wrong for patch files that name
  single parts by `id`.
- Apply the Elems to the parsed parts instead of the tree: an Elem can target any attribute of
  any element, which the parsed model does not keep.
