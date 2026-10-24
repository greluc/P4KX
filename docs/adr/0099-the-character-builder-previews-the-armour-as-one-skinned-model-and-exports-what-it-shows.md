# The Character builder previews the Armour as one skinned model and exports what it shows

Decided for issue #155 (M12.8). It builds on ADR-0078 (the Model tab's viewer, skinning and
playback), ADR-0079 and ADR-0082 (Armour on a body, Loadouts), ADR-0084, ADR-0090 and ADR-0091
(heads, their items and expressions) and ADR-0085 (the background build with Stop). The layout is
D.49 of the desktop spec.

## Decisions

- **The preview is the Armour the export writes.** The builder builds one `p4kx_assembly::Armour`
  from the choices and both the view and the GLB come from it, so the view never has a piece the
  file lacks and the report of pieces that could not be fitted is the report of the Armour.
- **One skinned scene.** `Scene::from_armour` flattens the parts at the level the Armour was built
  at into the scene of the Model tab: every skin is bound to the skeleton of the body by bone name
  (`BoneMatch`, as the GLB writer does), so the skeleton overlay and the playback of ADR-0078 work
  unchanged and the GPU code is not touched. The view draws no textures, like the Model tab; the
  materials choice only governs the export. Animations are not played yet; the builder owns a
  `Playback` without animations, which is where body and face animations plug in.
- **Expressions are a CPU morph of one mesh.** Expressions are loaded on demand (a checkbox), as
  they cost the rig blend of the head; the Armour is built with `Expressions::All`. Choosing one
  clones the vertices of the scene, moves those of the head mesh by weight times the sparse
  target and gives the scene a new serial, so the GPU uploads it as a new mesh. The strength moves
  in steps of 5 %. Normals are not recomputed.
- **Everything runs off the UI thread.** A change of body, head, piece, Loadout, detail or
  expressions starts a build that replaces the one that runs; the previous view stays until the new
  one arrives, and Stop works as in the Entities tab. The export writes `Armour::write_glb_with`
  on its own thread, counts the bytes written for progress, stops on cancel and removes the
  partial file.
- **Sources.** A head is a CHF file the Archive ships (listed from its entries) or one the user
  picks with a file dialog; the user's `CustomCharacters` folder is never read on its own. A CHF
  decides the body, because its head rig belongs to one. Pieces are the Entities of a worn item
  type; a Loadout is an XML Loadout of the Archive under `Scripts/Loadouts`, whose worn pieces the
  chosen pieces take the place of by item type (ADR-0082). Default Loadouts of Entities are not
  offered here; that needs the outfits of ADR-0082's open point.
- **Wrappers pass `file` on.** `ModelProvider::file` reads the DNA rigs; the wrappers the GUI puts
  around `ArchiveModels` forward it, or heads fail with a missing rig.
