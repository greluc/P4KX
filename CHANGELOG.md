# Changelog

Notable changes of P4KX, one section per Release. The section of a version becomes the
notes of its GitHub Release; the workspace version names the next Release, and `cargo xtask`
fails while its section is missing.

New changes are not written here but as fragments in `changelog.d/` (see its `README.md`), which
`cargo xtask changelog --release <version>` folds into the section of the Release together with
the entries under `## [Unreleased]`.

## [Unreleased]

## [0.3.0] - 2026-10-24

The third Release, and the first as P4KX: the tool and its repository are renamed (formerly
P4K-Extractor, now `greluc/P4KX`), and the settings folder moves to `P4KX` with a one-time copy.
The Basetool catalog becomes a full game-data export: ship fields, loadouts, paints, item stats
including armour, clothing, radar, tractor beams and salvage, variant groups, the starmap with
world positions, contracts, mineable elements and refining processes, and shops with their item
lists; the catalog is written as `.json.zst` for the upload. New tools: `p4kx coverage` and the
schema drift of `p4kx compare --schema`.

Two breaking changes: the Archive key is no longer part of the tool and is entered once, and the
Record XML namespace and the Record JSON schema `$id` now use `https://github.com/greluc/p4kx/`.

### Added

- Streamed and localised sounds. `p4kx wwise resolve` locates a streamed or prefetched WEM as the
  loose WEM Entry named by its ID below `Data\Sounds\wwise\` and resolves localised events,
  soundbanks and WEMs for `--lang <language>` with `--fallback-lang` (`English(US)` by default),
  both language folders read from the Archive; each variant states whether it is localised, the
  language it was resolved in and whether that is the fallback, and a localised sound without a
  WEM in either language is listed as a `no_localisation` broken link. `p4kx entity audio
  --sounds [--lang] [--fallback-lang]` lists the variants of every trigger with source and
  location. `ArchiveAudio::resolve_trigger_in`, `LanguageChoice`, `AudioIndex::languages`,
  `AudioIndex::loose_wem`, `WemLocation::Entry` in `p4kx-assembly`,
  `SoundSource::is_language_specific` in `p4kx-audio` (ADR-0033).
- Models: levels of detail, tangents and vertex colours. `p4kx model info` lists every LOD with
  its vertex and triangle counts and whether its meshes have normals, tangents and colours;
  `p4kx model glb --lod <n>` exports the level of detail `n` (0 by default; a level the model
  lacks exits with code 2) from the geometry companion `<stem>_lod<n>.<extension>m`, with the
  nodes of the base model file. The GLB now carries `TANGENT` (with handedness) and `COLOR_0` for
  the meshes that have them; `--no-vertex-colours` leaves the colours out. `Model::read_with`,
  `LodSelection`, `Mesh::tangents`, `Mesh::colours` in `p4kx-model`, `GlbOptions::vertex_colours`
  in `p4kx-gltf`. Real-Archive tests check the tangents against the texture axes, the LODs of 360
  models and the node chain against the `MeshInfo` box (docs/model-lods-and-vertex-attributes.md,
  ADR-0028). The vertex colours are read as BGRA; models whose names say `blue` are mostly blue
  and those named `red` or `orange` mostly red under that order, which a real-Archive test checks.
- Entry checksums: `p4kx verify <archive> [--filter <filter>]` checks the data of all or the
  filtered Entries against the CRC-32C of the Archive directory in parallel and reports every
  mismatch (exit code 1, `--json` lists them as `mismatched`); `p4kx extract --verify` and
  `p4kx socpak extract --verify` check each Entry while it is written, do not write a mismatching
  Entry and count it in the summary; `p4kx socpak verify` does the same with the plain CRC-32 of
  a container. The checksum is computed while the data streams, without a second pass, and
  reading stays unchecked unless asked for. `Archive::open_entry_verified` and
  `ExtractOptions::verify` are the library switches.
- socpak containers in the path tree of the window: a container expands like a folder, opened in
  the background with a spinner and cancelled by collapsing it, and nested containers expand too.
  Selecting a container lists its Entries for search, preview and the extraction of the selected
  or shown Entries. Opened containers are kept per Archive within 1 GiB of memory, closing the
  least recently used collapsed ones first; a corrupt container says so on its row, and a change
  of the Archive on disk closes them.
- Audio Conversion during Extraction: with `p4kx extract --convert` and the Conversion switch of
  the window, WEM sounds are written as Ogg (Wwise Vorbis and Opus without re-encoding, WAV for
  other codecs), as WAV or unchanged (`--audio ogg|wav|raw`, the same choice in the Extraction
  setup, remembered in the settings file). Each soundbank is written unchanged and unpacked into
  a folder of the same name, its WEMs named by ID and converted alike. A sound whose codec cannot
  be decoded is reported with the codec and written unchanged; the Extraction continues. Progress,
  cancellation and Conflict policies cover every converted file, a cancelled run lists the sounds
  of a soundbank it did not unpack, a soundbank folder exists only when a sound is written into
  it, and sounds are converted on their own threads within a memory budget.
- `p4kx ui render <archive> <entry> --out <file.png> [--size <W>x<H>|<W>] [--background
  transparent|movie|#RRGGBB[AA]]`: renders the first frame of a UI movie with its sprites as a
  PNG, drawn with tiny-skia. Shapes with solid, gradient and bitmap fills (external images and
  sub-images included) and strokes are drawn with the transforms, colour transforms and masks
  of their placements, morph shapes at their ratio. Texts, buttons and videos are skipped,
  filters and blend modes are not applied, and every element left out is listed with its
  reason, plain or with `--json` (exit code 1). Placements, paths, shape conversions, masks,
  gradient stops, sprite nesting, the canvas size and the area the fills cover (64 times the
  canvas) are bounded. Also for movie files on disk.
- `p4kx-mcp` DataForge tools: `get_record` returns a Record by GUID, or every Record of a record
  path, as Record JSON; `search_records` finds Records by record path and type name with Filters,
  in Record order; `list_structs` and `list_enums` list the struct definitions with parent and
  fields (data type, array, target struct or enum) and the enums with their values; `run_query`
  runs a Query and returns its typed columns and one page of rows as `p4kx dcb query --format
  json` writes them, the failed Records and the unresolved References, and an invalid Query as an
  error with its byte span. Every result pages with a cursor and holds at most 256 KiB of items;
  a single Record or row larger than that is an error. The DataForge is read once per open
  Archive, kept open with its indexes and read again when the Archive file changes. A Query is
  limited to 64 columns and 128 comparisons, selecting its Records stops after 30 seconds or when
  the client cancels, and the selection is kept for the pages that follow.
- `p4kx wwise resolve <archive> <trigger> [--json]`: resolves an audio trigger to its sound
  variants. Each Wwise event is followed through its Play actions and random, sequence, switch and
  blend containers to the Sound objects; every variant is listed with its soundbank, WEM ID,
  source (embedded, prefetched, streamed or plug-in) and container path, and an embedded WEM is
  located in the soundbank that holds it, with its offset and length. Broken links (a missing,
  unreadable or unfollowed object, a cycle, or a limit of depth, fan-out or size) are listed with
  the object where the chain breaks. Soundbank containers of version 154 are now decoded to their
  children.
- Query tab: completion of struct and field names from the DataForge schema while typing, after
  `from`, `.`, `->` and in `[…]`, with the field types; and saved Queries that are kept in the
  settings, run with one click, renamed and deleted.
- Looping sounds: `audio info` reports the loop of a WEM, `audio wav` keeps it in a `smpl` chunk
  and `audio ogg` in `LOOPSTART` and `LOOPLENGTH` comments.
- Soundbanks in the Audio tab: select a BNK Entry to see the WEMs it embeds as a list of ID,
  codec, duration and size, and select one to see its facts and play it with the controls of a
  loose WEM, without unpacking the bank. A WEM the bank holds only the prefetched start of and
  plug-in media are marked and explained instead of played; a corrupt soundbank shows a message
  and the rest of the window keeps working. The preview opens taller for soundbanks, and the list
  scrolls with the keyboard.
- Models: materials as plain colours. A model's `.mtl` file (text XML or CryXml) is read into a
  `MaterialLibrary` of sub-materials with shader, surface type, flags, colours, opacity, glow and
  texture slots; the material ID of a submesh is the position of its sub-material, and a file
  without sub-materials serves every ID. `p4kx model glb --materials colours` (the default)
  writes one glTF material per sub-material with base colour, opacity, roughness, metalness and
  emissive colour and no textures; alpha-tested materials mask, translucent ones, decals and
  glass blend, `MtlFlags` two-sided draws both faces, and submeshes whose material draws nothing
  (the `NoDraw` flag or shader, opacity 0) are left out. A library the model names with a bare
  name is looked for beside the model first. A missing, unreadable or unsafe `.mtl` path is
  reported by `model info` and `model glb` (text and `--json`) while the model exports with a
  grey default material, as does a material ID beyond the sub-materials or an entry of the
  sub-material list that is no material. `model info` lists the materials. `MaterialLibrary`,
  `MaterialStatus`, `Model::materials` in `p4kx-model`, `MaterialMode` in `p4kx-gltf`
  (docs/model-materials.md, ADR-0032).
- Models: textured PBR materials. `p4kx model glb --materials textures` embeds the textures of
  the materials in the GLB as PNG images, found through the Archive's path index (a `.tif` name in
  the `.mtl` file is the `.dds` Entry, with its alpha tail): the diffuse map (`TexSlot1`) as base
  colour, the normal map (`TexSlot2` or `TexSlot3`, `_ddna`/`_ddn`) with its blue channel
  reconstructed and its green channel flipped to glTF's up, and a metallic-roughness image whose
  roughness is the inverse of the normal map's smoothness (its alpha tail) and whose metalness
  comes from the specular map (`TexSlot4`); a metal's colour is its specular colour, so with a
  specular map the base colour holds the tinted diffuse and specular colours. `--texture-mip <n>`
  picks the mip level, 0 (the largest) by default. Masked materials use the texture's alpha.
  An image several materials use is embedded once; at most 256 images and 2^29 pixels per file.
  A texture that is missing, has an unsafe path, cannot be decoded or does not fit leaves the
  material's plain colour and is listed in the summary (`textures`, `textured_materials`,
  `texture_pixels` and `texture_fallbacks` in `--json`), names sanitised for the terminal.
  `write_glb_with` and `TextureLoader` in `p4kx-gltf`, `Texture::from_index` and
  `ModelSource::write_glb_textured` in `p4kx-extraction` (docs/model-materials.md, ADR-0042).
- Models: skeletons and skinning. `.skin` and `.chr` files are read with their bones (parent,
  bind pose relative to the parent and in the model's space, controller ID) and, for skins, the
  weights of up to eight bones per vertex; the `CompiledBones` form of static models with animated
  parts is read too. `p4kx model glb` writes a skin with its skeleton as a glTF skin: the bones as
  nodes, `JOINTS_0`/`WEIGHTS_0` (and a second set for vertices with more than four bones), inverse
  bind matrices, all in Y up and metres like the mesh; a `.chr` is written as its bone nodes. A level
  of detail whose own `_lod<n>` file has a different skeleton is weighted to that one. `model info`
  lists the bones with parent, node and bind position, and marks skinned meshes (new `Skinned`
  column; `--json` has `bones` with both poses and `skinned` per mesh); `model glb` reports the bones
  and skinned meshes. `Bone`, `BoneTransform`, `Influences`, `Mesh::influences`, `Lod::bones` and
  `Model::bones_of` in `p4kx-model` (docs/model-skeletons-and-skinning.md, ADR-0039).
- Models: `p4kx model glb --anim <name>` writes the animations of `.caf` files and `.dba`
  databases as glTF animations on the bone nodes of a skinned model or skeleton, or on the nodes a
  static model's helper bones drive (a door, a turret). `--anim all` takes every animation found in
  the model's folders, `--anim-from` adds files to look in. `p4kx model info` lists the animations
  with their duration. Tracks for bones the skeleton lacks are left out and reported. Additive animations (offsets
  to a base animation) are left out and listed as skipped unless named.
- Chunk files: `ChunkFile::chunk_data_to_end` returns the bytes from a chunk to the end of the file.
- Models: the older `CrCh` container. `p4kx model info|glb` reads nodes (name, hierarchy, parent
  relative transform), the physics mesh a `Mesh` chunk refers to as LOD0 with one submesh, and
  the material library from the `MtlName` chunk of a `CrCh` model, and reports unknown chunks as
  for `#ivo`. The Archives hold one such model, a Carrack collision proxy; only its chunk layouts
  are read (docs/crch-models.md, ADR-0041). `p4kx model` prints node and material names with
  control characters replaced.
- Window: a model file (`.cgf`, `.cga`, `.skin`, `.chr`) gets an Info tab next to Model that lists
  what the file holds: nodes, meshes, levels of detail with their vertices and triangles, the
  geometry file, the material library and the material IDs in use, the bones, the animations of the
  `.caf` files and `.dba` databases next to it, and the chunks counted as read, skipped and unknown
  with the ones the model did not read listed by file, type and type ID. It works for a model the 3D
  view cannot draw, such as a skeleton file. Export as GLB…
  (at the right end of the preview tab bar and in the Export menu) writes the selected model as one
  GLB file: a dialog chooses the level of detail, the material mode, the vertex colours, whether to
  write the animations of a model that has some (additive ones are left out) and the file,
  which cannot lie inside a Star Citizen installation. The export runs in the background with a
  progress bar and Cancel, writes the file only when it is complete, and ends with a summary of what
  the file holds, the animations written and the ones left out, the chunks it skipped, the materials
  that fell back to the default material and, with textures
  (choose "With textures" and a texture size), the textures the Archive lacks;
  a failure shows a message and the rest of the window keeps working, and a missing Archive key
  opens the key dialog. The choices are remembered in the settings file. `ModelSource::write_glb_observed`
  in `p4kx-extraction` writes with the animations, progress and cancellation
  (`ConversionError::Cancelled`) (ADR-0047).
- `p4kx entity glb <archive> <entity> --out <file>`: exports an Entity as one GLB, its geometry
  and the geometry of every Entity of its default Loadout placed at its port. Each part is a node
  named `<port>/<entity>` below the node of the Entity it is attached to, placed at the port's
  helper in the parent's model with the port's offset. `--lod <n>` applies to every model (a model
  without that level is written at the nearest lower one) and `--materials colours` writes plain
  colours, with the material library an Entity's Record names replacing the one its model names.
  Parts whose Entity or geometry cannot be resolved are skipped and listed in the report, as text
  or with `--json`. Models are read once and in parallel.
- `p4kx-assembly` builds an `Assembly`, a scene graph of placed models, from a Loadout tree, and
  `p4kx-gltf` writes a `Scene` of placed models as one GLB, writing equal materials once.
- 3D viewer tracer: selecting a model Entry (`.cgf`, `.cga`, `.skin`) opens the new Model tab of the
  Preview, which draws the model's most detailed level with wgpu inside the window: lit, framed
  to fit and shown beside a facts bar (vertices, triangles, size in metres). Drag orbits, a right
  or Shift drag pans, the wheel zooms; with the keyboard focus the arrow keys orbit, Shift and the
  arrow keys pan, `+` and `-` zoom and Home resets; **Reset view** frames the model again. The
  model is read and flattened on a background thread (at most 4,000,000 vertices and 8,000,000
  triangles, files up to 256 MiB), its buffers are uploaded once and replaced when another model
  is selected, and a device that cannot draw the view or is lost gets a message while the rest of
  the window keeps working. Materials, LOD and skeleton switches follow (ADR-0031, design
  decision D.35).
- `p4kx wwise events [--search <text>]` searches the audio triggers and Wwise events of the ATL
  definitions by name (substring or glob, ignoring case) and lists for each result the trigger,
  its events, the soundbanks that hold them and the Entities that name the trigger. The output is
  plain or `--json`, in definition order and paged with `--limit` and `--offset`; `--entities`
  bounds the Entities listed per trigger and `--no-entities` skips the DataForge.
- `p4kx ui render … --svg`: writes the first frame of a UI movie as an SVG document that scales
  without loss, for the movie file or an Entry. Shapes become `<g>` elements placed with `<use>`,
  gradients, strokes, colour transforms and masks become the matching SVG elements, and bitmaps
  are embedded as PNG data; the output is deterministic, streamed to the file, and bounded by the
  limits of the PNG, so both end at the same element, and at most 1 GiB. The title is the Entry
  name, or the file name without its directory, escaped. A bitmap fill that does not repeat is
  drawn once, without continuing its edge pixels, and a stroke painted with a non-repeating
  bitmap tiles it. The `--json` report of `ui render` names the written `format`.
- `p4kx ui render`: static texts (`DefineText`) and text fields (`DefineEditText`) are drawn from
  the glyph outlines of the embedded fonts, with their size, colour and alignment, in the PNG and
  as paths in the SVG. Text fields honour the Flash HTML subset (`<p align>`, `<font face size
  color letterSpacing kerning>`, `<br>`), word wrap, margins, indent, leading and password
  fields. A character whose font or glyph is missing is drawn as an empty box and listed in the
  report (`missing_font`, `missing_glyph`, `missing_glyph_index`); no system font is used. With
  `--lang <language>` and an Archive, localisation keys (`@key`, with `,P=` parameters) resolve in
  that language's `global.ini`; an unknown key is drawn as it is and listed (`unlocalised`). A
  text field shows at most 4096 characters and a frame lays out at most 131072; the rest is
  listed (`text_truncated`, `too_many_glyphs`).
- Incoming References: `p4kx dcb refs <archive> <guid|record path> --incoming|--outgoing [--format table|json]`
  lists the Records that refer to a Record, or those it refers to, with the field and the array
  indices of every Reference, the References to a GUID no Record has apart; `--files` lists the
  edges between record files, `--unresolved` every Reference to a GUID no Record has. The window
  shows a *Referenced by* tab for the selected Record, a Query selects Records with `referencing
  "<GUID or record path>"`. One index over all Records, built in parallel on first use and kept
  per Archive, serves all of them.
- Content search: `p4kx grep <archive> <text> [--in records,loc,entries] [--filter <filter>]
  [--lang <language>,…] [--limit <n>] [--per-item <n>] [--format table|json]` finds a text, ignoring
  case and as part of a word (`Gladi` finds `AEGS_Gladius`), in the string, localisation key and
  enum values of all Records, in the localisation texts of each language and in the lines of text
  and CryXml Entries. A hit names the Record and its field path, the language and key, or the
  Entry and its line, with a short excerpt. Binary and oversized Entries and encrypted Entries
  without the Archive key are skipped and counted. The window gets a *Search* tab with the same
  three sources: a click on a hit selects the Record, opens the localisation key or selects the
  Entry. Each search scans in parallel without an index (ADR-0044); on LIVE it takes about a second
  for all Records, 0.15 s for all localisation texts and two seconds for all text Entries.
- Field statistics: `p4kx dcb fields <archive> [<type or Query>] [--path <glob>] [--format table|csv|json]`
  lists every field path of the Records, e.g.
  `Components/SCItemWeaponComponentParams/fireActions/SWeaponActionFireRapidParams/fireRate`, with
  how many values it has, in how many Records, of which kinds, how many are empty or distinct, and
  the smallest and largest number, to find the field a Query needs. The target of a strong pointer
  adds its actual struct to the path. One parallel pass counts all Records of LIVE in about two
  seconds.
- CLI: `p4kx coverage <archive>` checks a reference list of the game data a companion app reads
  against the Archive: for each Record type, component of an `EntityClassDefinition` and vehicle
  definition folder whether it exists, how many Records, instances or Entries it has, and whether
  they read, export and convert. The list is data in the repository; the result is a table or
  JSON ([docs/coverage.md](docs/coverage.md)).
- `p4kx ui render`: fonts that a UI movie imports from another movie (`ImportAssets`,
  `ImportAssets2`), which most text fields of the game name, are resolved in the Archive and the
  texts are drawn with their glyphs instead of boxes. The exporting movie is found from the import
  URL relative to the importing movie, with `.swf` and `.gfx` as alternatives and the UI and
  English localisation folders as fallbacks, read once per render, and followed through chains of
  imports up to 4 levels deep; the report lists the movies read and the fonts not found
  (`imports`). On the LIVE Archive 98.6 % of the text fields with imported fonts resolve, and the
  first frames of the movies with text lose 1388 of 1424 `missing_font` entries.
- `p4kx ui assets`: lists the imports (`imports`), symbol classes, nine-slice grids, the frame index
  of every frame label (`frame_labels`) and the known tags whose content is not read
  (`unread_tags`).
- Entity: `p4kx entity loadout` reads the vehicle implementation XML of a ship with the
  Modification its Record selects. The tree shows per port whether the implementation hides it
  (`invisible`), and lists the ship's treads, wheels and landing gear; the JSON has `vehicle`,
  `landing_gear` and a port `visibility`. `p4kx-assembly` offers `VehicleImplementation::read`
  for exports.
- `PortNode::nested` in `p4kx-assembly`: the ports of the nested Loadout of an empty port.
- Loadout: docked vehicles (attached Entities whose item type starts with `NOITEM_Vehicle`, such
  as snub craft) are marked in `p4kx entity loadout`, in the text and as `docked_vehicle`,
  `expanded` and `docked_vehicles` in the JSON. `--stop-at-docked-vehicles` leaves their ports
  out; `LoadoutResolver::stopping_at_docked_vehicles` does the same for library users.
- Entry SHA-256: every Entry of the Archives measured states a SHA-256 of its stored bytes (the
  `0x5003` block of the v1 directory), which `Entry::sha256` returns and `p4kx verify` and
  `--verify` check in addition to the CRC-32C, while the data streams. It covers the bytes before
  decryption and decompression, so it needs no key; a mismatch is reported with the kind `sha256`
  and 64 hex digits (ADR-0034, docs/verify.md). The 32 bytes of a v2 record are read and returned
  by `Entry::sha256` but not checked until a real v2 Archive confirms the layout. `Checksum::hex`
  was added in `p4kx-archive`.
- DataForge: the tag tree. Fields such as `Tags`, `RequiredTags` and `PortTags` refer to Tags by
  GUID; `TagTree` in `p4kx-dataforge` reads the `TagDatabase` into the tree of Tags with their tag
  paths (`Ship.Series.Avenger`), parents and children, and answers tag GUID to tag path. A Tag the
  `TagDatabase` does not reach is a root of its own; Tags listed twice, dangling children and
  cycles are counted and never fail (ADR-0052). The tree is reachable from `p4kx-extraction` and
  `p4kx-export` for the Basetool catalog kinds that match tags.
- `p4kx dcb tags <archive> [<tag>] [--tree] [--format table|json]` prints the tag tree, or one Tag
  and everything below it; `--tree` indents the names or nests the JSON. `p4kx dcb export --tags`
  writes the tag path next to every Reference to a Tag, as the `tag` attribute of Record XML and
  the `tag` key of Record JSON (`RecordExportOptions::tags`, `Annotations`); without it the output
  is unchanged (docs/record-xml.md, docs/record-json.md, docs/cli-json.md).
- The Tags tab of the window shows the tag tree with GUID and legacy ID, expands and collapses
  Tags, searches the tag paths and copies the path or GUID of the selected Tag.
- Comparison: `p4kx compare <old> <new> --schema` lists the structs, properties (data type, conversion type, target), enums and enum options that were added, removed or changed between the DataForges of two Builds, with the number of Records each concerns; `--json` prints the report, whose JSON Schema is `schemas/schema-drift-report.schema.json`. The Comparison tab shows the same drift in a new Schema list once the Record level was compared.
- Ship base data: `p4kx ships <archive> --out <file>` writes the hull mass, the hull points with
  the destruction and detach thresholds of the parts, the cargo capacity of the cargo grids of the
  default Loadout, `AttachDef.Size` and the default Loadout as class names per port path of every
  ship as JSON or NDJSON, each value with the file and field it was read from (the Record, the
  vehicle implementation, the Modification's patch file or its `Elem`). Docked vehicles are marked
  and not counted. `build_ship_base` and `ShipReader` in `p4kx-export`;
  `VehicleImplementation::totals`, `ValueOrigin` per part value, `VehiclePart::destroy_ratio`,
  `EntityNode::loadout`, `EntityNode::attach_type` and `EntityNode::inventory_container` in
  `p4kx-assembly` (docs/ship-base-data.md, ADR-0061).
- Basetool catalog: every weapon, shield, quantum drive, power plant, cooler and missile in `items`
  carries typed stat blocks in a new `stats` field, as raw Archive values without derived
  figures (ADR-0065). The catalog stays at version 1 (ADR-0054).
- Basetool catalog: the new kind `starmapObjects` lists the starmap objects (star systems, planets,
  moons, stations, jump points) with `type`, `parentGuid`, the localised `name` and the world
  `position` in metres. The position composes the `pos` and `rot` of every `Child` element of the
  system containers, with the full rotations of the containers above it. Additive; `schemaVersion`
  stays 1 (ADR-0054). `p4kx catalog` prints the placement counts and warns about containers it
  cannot read (exit code 1).
- The Build carries the public patch version (for example `4.7.2`) when the RSI Launcher log has a
  line with the Build's change number. `p4kx info`, `p4kx channels --json` and the Basetool
  catalog's `source.build.publicPatch` show it; it is `null` when the log has no such line.
- Basetool catalog: every ship and item carries `variantOf`, the GUID of the base of its cosmetic
  variant group, or `null` (ADR-0066). `p4kx catalog` counts the groups and variants per kind. The
  catalog stays at version 1 (ADR-0054).
- Legal notices: README, the new window "About P4KX" under Help, `p4kx --version` and
  `--help`, the `p4kx-mcp` help and instructions, and the notes of every Release state that
  P4KX is an unofficial Star Citizen fan project, not affiliated with or endorsed by the
  Cloud Imperium group of companies, ships no game data, reads only your own installation, and
  that you are responsible for complying with the Star Citizen EULA and Terms of Service. README
  and the About window also carry the trademark notice of the Fankit (text only; in the English
  wording in German too).
- MCP server: the README, the server instructions and the descriptions of `read_entry_text`,
  `read_entry_bytes`, `get_record` and `run_query` say that returned content may be forwarded by
  the MCP client to third parties such as a cloud model provider, and that game texts and media
  must not be redistributed.
- Read-only use: Extraction, Export and every other write refuse a target folder inside a Star
  Citizen installation (the folder holding the opened Archive and the Library folders), however
  the path is spelled. While `StarCitizen.exe` runs, the window and the command line warn that the
  tool should be used with the game closed; reading continues. Only the process list is read.
  The whole installation counts, including a Channel's `user` folder, so `p4kx chf from-json` no
  longer writes into `CustomCharacters`: copy the file in by hand.
- Archives of test Channels (PTU, EPTU, TECH-PREVIEW, HOTFIX): the window shows a persistent
  warning notice, `p4kx` prints one line on stderr and `p4kx-mcp` adds a note to its responses that
  the material of test builds is confidential under the Star Citizen Terms of Service and must not
  be shared or published. Opening, browsing, extracting and comparing work as before (ADR-0038).
- Basetool catalog: every ship and item carries `playability` (`player`, `ai`, `mission`,
  `special` or `null`), decided from crew manifests, tags, the inclusion mode and the shop display
  of the Records, never from names. `p4kx catalog` prints the count per value and `--json` lists
  them under `playability`. The catalog stays at `schemaVersion` 1.
- Basetool catalog: the kind `loadouts` lists, for every ship and every item that has ports, the
  ports with size, item types, flags, tags and visibility, and the Entity the Default Loadout puts
  into each as an `itemGuid` into `items` or `ships`, with the ports of the attached Entity nested
  below it (the Loadout's overrides applied). Docked vehicles are marked and not expanded. The kind
  is additive, `schemaVersion` stays 1 (ADR-0054, ADR-0064). `p4kx catalog` and `--json` count the
  ports, the attached Entities and those without an entry in `items` or `ships`; the window lists
  the kind (docs/basetool-catalog.md).
- Basetool catalog: the kinds `resourceGroups` (the `ResourceTypeGroup` Records, with the group that
  lists each as `parentGuid`) and `blueprintCategories` (the `BlueprintCategoryRecord` Records).
  Every commodity `typeGuid` and every blueprint `categoryGuid` now resolves to an entry. The
  kinds are additive and keep `schemaVersion` 1 (ADR-0054). `p4kx catalog` and the Basetool
  catalog summary of the desktop app list their counts.
- Basetool catalog: every ship carries `mass`, `massBasis`, `hullHealth`, `cargoScu`,
  `cargoGridsWithIssue`, `size` and `vehicleKind` (spaceship or ground vehicle) from the ship base
  data. Unknown values are `null`, never 0. `p4kx ships` states the `vehicleKind` too.
- Basetool catalog: every blueprint lists all its `tiers` with the selections ("choose n", with
  `count` and `options`), the quantity multiplier per cost line and the quality modifiers, next to
  the unchanged first-tier fields `craftTimeSeconds` and `ingredients`; it also states
  `defaultAvailable` and its `dismantle` link. The new kinds `gameplayProperties`,
  `qualityDistributions` (with their location overrides), `blueprintPools` (with the places that
  reward them and their chance) and `dismantleProcesses`, and `craftingGlobal` with the resources
  and items a dismantle excludes, complete the crafting data. All additive; `schemaVersion` stays 1
  (ADR-0054). `p4kx catalog` lists the counts and warns when the reference index is truncated or
  holds several dismantle processes (exit code 1).
- Crafting library: `Crafting::warnings` reports a truncated reference index, the Records it could
  not read and several dismantle processes instead of dropping reward sources silently;
  `dismantle_return()` links only the single dismantle process and exposes the tier and
  `excluded_resources`. The catalog and the library select blueprints by one rule.
- Basetool catalog: every commodity carries `densityGramsPerCubicCentimetre` and
  `refinedSubtypeGuid` from its `ResourceType` (additive in schema version 1). The DataForge holds
  no yields; see `docs/commodity-refining.md`.
- Basetool catalog: the kind `paints` lists every paint (an item of the type `Paints`) with the
  ships it fits, `shipGuids`, found structurally: the paint port of a ship (`SItemPortDef` of its
  Record or `ItemPort` of its vehicle implementation, with the Modification applied) requires tags
  that the paint's `Tags` carry or lie below in the tag tree. The kind is additive, `schemaVersion`
  stays 1 (ADR-0054). `p4kx catalog` and `--json` count the paints that fit a ship and those that
  fit none; the window lists the kind (docs/basetool-catalog.md).
- Basetool catalog: an output name ending in `.zst`, for example `p4kx catalog <archive> --out
  basetool-catalog.json.zst`, writes the catalog as one zstd frame (level 19, content size and
  checksum declared, so a plain `zstd -d` decodes it), compressed while it is serialised. The desktop save dialog offers the
  second file type, and the summary shows the compressed and the uncompressed size. The size
  warning compares the compressed file with the 64 MB upload limit and the decompressed catalog
  with the 1 GB limit of the Basetool; `p4kx catalog --json` gains `compressed_size`,
  `decompressed_limit` and `decompressed_warning`. Plain `.json` stays the default.
- Basetool catalog: the new kind `contracts` lists every contract and sub contract with its
  resolved title, mission type and the reputation it grants on success. Version 1 stays.
- Basetool catalog: the new kinds `mineableElements` and `refiningProcesses`. A mineable element
  carries `resourceTypeGuid` (a commodity's `subtypeGuid` only when the catalog has that commodity;
  20 of 41 on LIVE), `instability` and the other plain numbers of the `MineableElement` Record;
  several elements may share one `ResourceType`, so none is picked per commodity. A refining process carries `processName`, `refiningSpeed` and
  `refiningQuality`; the DataForge holds no yields. Additive; `schemaVersion` stays 1 (ADR-0054).
- Basetool catalog: the new kind `shops` lists the shops of the registry `LayerBackups.xml` (Stanton
  and Pyro) with their identity (`key`), internal names, the container chain, `body` and `location`,
  `kind` and `brand` from the shop containers, a link to the starmap object of the location
  (`starmapObjectGuid`) and a flag for commodity exchanges. It holds no prices, stock or item list;
  the inventory files cannot be assigned to shops (ADR-0069). Additive; `schemaVersion` stays 1
  (ADR-0054). `p4kx catalog` prints the shop counts.
- Basetool catalog: a shop carries `inventory` and `itemGuids` when its shop containers name an
  inventory file in their `.soc` (ADR-0071); 331 of the 1,184 shops on LIVE get a list. It holds the
  item GUIDs only, no price, stock or rental data. `p4kx catalog` counts the shops with and without
  a list and warns about inventory files it cannot read.
- Basetool catalog: `stats` of an item now also covers suit armour (`armor`), clothing and
  armour worn values (`clothing`), radars (`radar`), tractor beams (`tractorBeam`) and the salvage
  and tractor multipliers of attachments (`weaponModifier`), and every fire mode has a
  `damagePerSecond` (the power of a mining laser) and a `spread`. The document states, for each
  gameplay property a blueprint modifies, the `stats` field it changes or why there is none
  (ADR-0072). The catalog stays at version 1.

### Changed

- The window, the installer and the documentation no longer name an organisation. The look is
  unchanged; the header shows the wordmark only, and the P4KX glyph and the window icon built from
  it are removed (the platform's default icon shows). The
  copyright reads "Copyright (c) 2026 the P4KX contributors", the installer manufacturer is "P4KX
  contributors", and the installer writes `HKCU\Software\P4KX` instead of a key below the earlier
  organisation path; an installed version upgrades and its old key is removed.
- **Breaking:** the Archive key is no longer part of the tool; you enter it
  ([ADR-0027](docs/adr/0027-archive-key-comes-from-the-user.md)). The constant `ARCHIVE_KEY` of
  `p4kx-foundation` is gone: `ArchiveKey` is a value that `Archive::open_with_key` or
  `Archive::set_key` give an Archive, and `p4kx_testkit` encrypts its Archives with a test key
  of its own. Without a key an Archive still opens, lists, searches and compares, and reads every
  unencrypted Entry; an encrypted Entry fails with `ArchiveError::KeyRequired`, and a key that
  does not decrypt the Archive with `ArchiveError::WrongKey`, found by decrypting one encrypted
  Entry and comparing its CRC-32C. `p4kx --key <hex>`, the environment variable
  `P4KX_ARCHIVE_KEY` and the key stored with `p4kx key set|show|remove` in your profile give
  the key, in that order; the MCP server takes the same three (`--key`, `P4KX_ARCHIVE_KEY`, the
  stored key), and the window asks in a dialog when an encrypted Entry is first needed, and from
  **Archive key…** in the Archive menu. A missing or wrong key exits `p4kx` with the new code 7.
  The key is stored in `archive.key` next to the settings, written atomically and readable by
  your user alone where the platform allows it, and never appears in logs, error messages or JSON.
  `cargo xtask keys` fails when a tracked file holds a key-like literal named as a key, or the
  real key while `P4KX_ARCHIVE_KEY` is set. Real-Archive tests need `P4KX_ARCHIVE_KEY`.
- Models: `p4kx_model::ModelFile` is `#[non_exhaustive]` and has the new variant
  `LodGeometry(u32)` for the geometry companion of a level of detail from 1 on (`lod<n>-geometry`
  in the `chunks` of `model info`), so a `match` on it needs a wildcard arm. The table of
  `p4kx model info` has the columns Tangents and Colours, and its chunk table is wider for the
  `lod<n>-geometry` names.
- GUI: the window follows the measured palette: the house orange is `#E77E23` again, the canvas `#0F0F0F`, the panel `#282C30`, the
  bar `#1A1D21`, the control `#373E43` (hover `#434B51`), the hairline `#36373B`, the count badge
  `#2F394C`, the text `#D1D2D3` and the muted text `#A2A5A9`; the selected tint is 26 % orange,
  every orange tint and glow follows, tooltips fill with the control colour, and the glyph and
  logo SVGs use the house orange, tree rows hover in 2.5 % white and the drop target fills with
  6 % orange as the CSS of the design sets them. The earlier values were sampled from HDR screenshots and were
  too light.
- GUI: the window follows the design after an adaptation to a website palette: house orange `#FB8A28`, the canvas `#202020`
  with panel `#35393D`, bar `#2A2D31` and control `#3F474C` fills, the lighter text and border
  greys, the new text tints, and the `radius-sm` corner of 4 points on buttons, fields, segments,
  chips, alerts, modals, dropdowns, tiles, the list panel and the preview. Header, status bar and
  tab bar use the bar surface, table rows hover in `control-bg` and tree rows in 3 % white, the
  ghost button hovers in `color-accent-light`, tooltips are square on near-black with the orange
  glow, focus rings follow the radius of their control, and the range thumb is round. The empty
  state no longer hides the work tabs, the dialogs' foot no longer stretches the dialog to the
  window height, and read-only and text fields draw the rounded input frame. The desktop spec and
  its JSON follow.
- Folders whose names differ only by ASCII case, or whose Entries use `/` and `\` alike, are one
  folder, in the window's tree as in `list_entries` of `p4kx-mcp`, named as its first Entry in
  path order spells it (ADR-0023). `list_entries` lists the Entries of a folder in path order and
  pages in time proportional to the page; the window and the server share one folder index.
- DataForge: a format drift that no known database has is read and reported instead of failing the
  Record or the file. Bits above the low byte of a property's conversion type are ignored, so a
  flag no longer turns a single value into an array slot and breaks the struct layout; a Record
  whose name offset is `0xFFFFFFFF` reads with an empty name and one whose tag offset is
  `0xFFFFFFFF` without a tag. `DataForge::issues` lists these deviations (one per property or
  Record, at most 1024 listed, all counted by `issue_count`), and `DataForge::survey` counts the
  format details no tool interprets (docs/dataforge-format.md). A real-Archive canary test
  checks them for `P4KX_DATA_P4K` and `P4KX_DATA_P4K_B`.
- Three `#ivo` chunk types change their name in `p4kx chunks list` and `p4kx model info`, also in
  the `type` field of their `--json` output: `0x2B7ECF9F` is `PositionBonemap` (was
  `BoundingData`), `0x58DE1772` `StatObjPhysics` (was `LodMeshData`) and `0xBE5E493E`
  `ExportFlags` (was `AssetMetadata`); `0x57A38888` is now `BShapesGpu` and `0xF5C6EB5B`
  `BShapes`.
- Models: the `TANGENT` attribute of a GLB has the handedness `w` negated, so that the bitangent
  points up the image as glTF's does; the engine's points down it, and the normal maps of
  textured materials have their green channel flipped to match (ADR-0042, which settles the open
  question of ADR-0028).
- GUI: alerts fill the width of their panel like the theme's `.alert`, and the tabs of
  the preview scroll sideways with the mouse wheel when they do not fit next to Back, Forward and
  Copy, like `.tab-nav` with `overflow-x: auto`.
- `p4kx ui render`: a font without any glyph outline is a missing font (`missing_font`, drawn as
  boxes) instead of drawing nothing, and fonts of other movies are found by name only, never by
  their own ID.
- `p4kx entity glb` places a docked vehicle, such as the Merlin of a Constellation or an
  Archimedes, as one part without the parts of its own Loadout, so nothing is counted twice.
- `p4kx-archive`: `Checksum::value` now returns `Option<u32>`, `None` for a SHA-256 digest, which
  breaks library users that relied on a plain `u32`.
- Changelog fragments: a user-visible change adds a file `changelog.d/<issue>.md` instead of an
  entry in `CHANGELOG.md`, so parallel pull requests no longer conflict there. `cargo xtask
  changelog` checks the fragments, and `cargo xtask changelog --release <version>` folds them and
  the entries under `## [Unreleased]` into the section of the version (ADR-0035).
- Basetool catalog: new kinds and fields may be added to version 1 without raising `schemaVersion`
  (ADR-0054), and a new value in a closed value set is additive. The JSON Schema now allows unknown
  properties, types `itemClass` as a string, and marks every game-text field with `x-gameText`. The
  documentation lists the guarantees for readers and says that a production import takes `LIVE`
  catalogs only.
- Basetool catalog: the window suggests `basetool-catalog.json.zst` as the file name and lists the
  compressed form first. Plain `.json` stays selectable but is over the 64 MB upload limit; a
  warning advises `.json.zst`.
- Breaking: the Record XML namespace URI is now `https://github.com/greluc/p4kx/record-xml` and
  the `$id` of the Record JSON schema is
  `https://github.com/greluc/p4kx/schemas/record-json.schema.json`. These replace the
  identifiers of earlier versions in one step. A consumer that checks the old
  namespace or schema id must switch to the new one. The update check, the About text and the
  installer link point to the new repository.
- The tool is now called P4KX, and its repository is `greluc/P4KX`. The window, the About text,
  the help, the installer and the artefacts (`P4KX-<version>-x86_64.msi` and the portable ZIP)
  carry the new name. The MSI keeps its upgrade code, so an installed P4K-Extractor upgrades to
  P4KX; its install folder, Start menu shortcut and `PATH` entry move to the new name.
- Converted audio carries `P4KX` in the Ogg and Opus vendor string, so the bytes of converted
  files differ from earlier versions. The generator of exported glTF files reads `P4KX <version>`
  and the update check identifies itself as `P4KX/<version>`.
- The settings folder is now `P4KX` in the platform config directory (`%APPDATA%\P4KX` on
  Windows). On first start, when it does not exist yet, the content of the old `P4K-Extractor`
  folder (the settings and the stored Archive key) is copied into it; the old folder stays as a
  backup, and the copy never overwrites a file. When the copy fails, the old folder stays in use
  and the next start tries again.
- Basetool catalog: shops get their item list from shop components in every container and from the
  shop prefabs, not only from the shop folders, and the components that later positions of a
  container file hold are found by object name or `CryGUID` (ADR-0073). For the shops that name the
  placeholder inventories `temp_outpost_shopinventory` and `inv_cryastro_dummy`, `inventory` and
  `itemGuids` are now `null` instead of the placeholder list. On LIVE 294 of 1,184 shops have a
  list, 64 are unscanned instead of 144. The shape of the catalog does not
  change; `p4kx catalog --json` counts the new cases in `shops`.

### Removed

- `ModelError::UnsupportedContainer` in `p4kx-model`: `CrCh` models are read.

### Fixed

- Archives with a malformed field in the directory record of a single Entry now open instead of
  failing as a whole, and the Entry is reported as an Entry issue (ADR-0025): an Entry path that
  is not UTF-8 is listed with its invalid bytes replaced and extracted under a name that spells
  them as `%XX`, so Entries with equal replaced paths no longer overwrite one another; an
  encryption field or 0x5002 block value other than 0 or 1 is read as encrypted, and a 0x5002
  block of another length than 6 bytes is reported; a v2 Entry whose data lies outside the file is
  reported when the directory is read and fails only when it is read. `p4kx info`, `list` and
  `extract` print the issues to stderr, `p4kx info` counts them, and `archive_info` of `p4kx-mcp`
  lists them; at most 1024 issues are listed and all are counted. A ZIP64 locator that does not
  stand right before the end record is found by searching backwards. Extraction now also rejects
  the Windows device names `COM¹` to `COM³` and `LPT¹` to `LPT³` as Entry paths.
- CHF: a file whose body changes keeps the last 8 bytes of the original file (the game writes
  `efbeadde efbeadde` there; all 41 CHF files of the LIVE Build have it) instead of zeroing them,
  with a correct CRC-32C. A new frame that would reach into non-zero ones is an error, not an
  overwrite. An unchanged JSON still gives the original file byte for byte (ADR-0014).
- Localisation: `@blank_space`, the filler Records use for an empty line (372 fields in the LIVE
  Build), is a placeholder key like `@LOC_EMPTY`: it has no text, so `p4kx loc get` reports
  `placeholder` instead of the single space of the English `global.ini`, and `p4kx dcb export
  --lang` no longer annotates it. A `global.ini` that holds both `key=` and `key,P=` keeps both
  lines; a lookup prefers the plain key (`GlobalIni::resolve_suffixed` returns the other). The
  Basetool catalog of the LIVE Build is unchanged.
- Soundbanks of a version above 154 are no longer misdecoded: their layouts differ (an Event
  starts with other fields), so a Sound, Action, Event or container of such a bank is kept raw and
  reported as an object whose layout is not known (`docs/audio-bank-versions.md`). `HircKind::of`
  maps the type numbers of `HIRC` objects by bank version, which differ up to version 126. Star
  Citizen's banks are all version 154 and decode as before.
- A Sound object of a version 150 soundbank is decoded correctly: it has no `cacheID`, which
  versions from 151 on write before the in-memory media size, so the media size was read one field
  late. Star Citizen's version 154 banks were not affected.
- A WEM with the format tag `0x0002` is no longer always taken for IMA ADPCM: a `fmt` chunk that
  holds DSP coefficients (`12 + 46 × channels` bytes after the common fields) is DSP ADPCM, which
  is reported as unsupported, and one with a block alignment of 260 bytes per channel is PTADPCM
  and decodes. These are console variants; every Star Citizen WEM is Wwise Vorbis.
- Soundbanks with a quirk that leaves the rest readable are no longer rejected whole: bytes after
  the last `HIRC` object, a repeated `DIDX` WEM ID (the first entry is kept), `DIDX` without
  `DATA` and a non-UTF-8 name in `STID` are worked around and listed by `Soundbank::warnings`.
  The audio index reads soundbanks up to 32 MiB instead of 16 MiB, and `Wem::chunks` and
  `Wem::chunk` list every RIFF chunk of a WEM, `smpl`, `cue ` and `LIST` after `data` included.
  No soundbank or WEM of the LIVE, PTU and TECH-PREVIEW Archives has any of these quirks
  (`docs/audio-survey.md`).
- Mesh dequantisation: a survey of LIVE, PTU, TECH-PREVIEW and EPTU compares the two boxes of
  the mesh chunks (static models only; skins store floats). Of the about 30,000 `.cgfm` chunks
  and about 31,000 `.cgam` chunks of each Archive whose boxes differ, the face normals can decide
  about 6,000 in LIVE, and 99 % of those choose the second, scaling box; the rest are axis-aligned
  and undecidable. The reader keeps the second box for every mesh, documents the evidence and its
  limits (docs/mesh-bounding-boxes.md) and tests it with a differing model box; no exported model
  changes.
- Mesh streams: both normal tags now take 4-byte packed and 12-byte float vectors, and a
  16-byte tangent stream (two quaternions) gives normals, so a mesh that stores only such a
  stream keeps its normals; no real file has the 16-byte layout, so reading the normal from its
  first quaternion is unverified. The packed vectors of the second normal tag rank last: in LIVE they
  disagree with the tangent frames of the 17,902 meshes that hold both, so no exported model
  changes.
- Mesh streams: a mesh chunk that holds one of the streams a model reads (indices, vertices,
  normals, tangents) twice is reported as corrupt (`RepeatedStream`) instead of the later stream
  silently replacing the first. No mesh chunk of LIVE, PTU, TECH-PREVIEW or EPTU does.
- The `#ivo` chunk type table follows a survey of LIVE, PTU and TECH-PREVIEW
  (docs/chunk-types.md): twelve types that `p4kx chunks list` reported as unknown are named
  (`BShapes`, `BShapesGpu`, `RigLogic`, `RigInfo`, `ProtosInfo`, `PhysicalHierarchy`, `VisAreas`,
  `Skeleton` and four more), seven stay unknown, and the table entries that occur in no measured
  Archive are marked unverified.
- A WEM whose odd-sized chunk is not followed by a pad byte, as Wwise writes it, no longer
  misaligns the chunks after it: the reader takes the position right after the chunk when it
  holds a valid chunk header, and skips the pad byte of plain RIFF otherwise. No WEM of the LIVE,
  PTU and TECH-PREVIEW Archives has an odd-sized chunk before its last.
- Models: a skin with a texture coordinate that is NaN or infinite (14 `.skinm` of LIVE, helmets
  among them) no longer fails to open; the coordinate reads as 0.
- Models: a model whose geometry companion holds only visibility areas (a `VisAreas` chunk and no
  mesh, as in the interior visarea corridors of ships) reads as helper nodes without meshes
  instead of failing with a missing mesh; `model info` lists the chunk as skipped.
- Loadout: an entry without an Entity (an XML `<Item>` with an empty `itemName`, or a DataForge
  manual entry without target) that carries nested entries no longer drops them. The port stays
  empty and the nested entries appear below it as `PortNode::nested` in `p4kx-assembly`. This
  changes the output of `p4kx entity loadout`: the text lists them indented below the empty port,
  the JSON gives every Port a `nested` array, and `ports`, `attached` and `empty` count them. On
  LIVE it affects the two human body Entities (`Body_ItemPort` with 10 nested items) and
  no ship or vehicle.
- `THIRD-PARTY-NOTICES` now carries the licence text and copyright notice of every Rust crate
  linked into `p4kx`, `p4kx-gui` and `p4kx-mcp` (ISC, Unicode, MPL-2.0, Zlib, OFL and the other
  licences that were missing), plus the Lato fonts. CI fails when a dependency change leaves the
  file out of date (`cargo xtask notices`).
- Vehicle implementations: a number with an invisible Unicode format character inside it, such as
  the U+202C in the Carrack's `mass`, is read in full instead of up to the character.
- Query: a completion request made right after the Archive changed on disk is no longer ignored
  while the build of the schema for the previous DataForge is still running.
- `p4kx coverage`: `SolarSystem` is counted as the `StarMapObject` Records of that
  `StarMapObjectType` (new `starmap-type` kind and `starmap_types` JSON array) instead of as a
  missing Record type, so the exit code no longer reports a gap that does not exist.
- Catalog: ships whose vehicle definition differs only in case or path separator now fall into
  one variant group. Items with weak pointers into different instances of their Record no longer
  compare equal, and a repeated GUID in a malformed Archive no longer counts as a group without
  variants.
- Catalog: contracts whose `ContractTemplate` Record lies outside `contracts/contracttemplates`
  now take title and mission type from it. An organisation token without a variant is resolved
  only for an organisation with a single string variant. A cancelled build stops while the
  contract records are read, and a Filter that matches no generator no longer reads them.

## [0.2.0]

The second Release: readable game data. CryXml and DataForge Records as XML and JSON, the
Basetool catalog, the Comparison of two Archives, textures and localisation, and first commands
for Queries, audio, models, UI movies, socpak containers, CHF files and the MCP server.

### Added

- Verification of textures and localisation against all installed Channels, recorded in
  `docs/m5-verification.md`. A texture header Entry that ends with the 8-byte CryEngine trailer
  `CExtCEnd` after its last mip (`cm_inventory_probe_cm.dds`, in every Channel) is no longer
  refused with "not a whole number of mips": the trailer is left out of the merged DDS and of
  the mips read. Opening a texture only checks the trailer's size; `merge` and `read_mip` of a
  mip from the header Entry's tail verify its bytes (`TextureError::UnknownTrailer`), `read_mip`
  of a mip from a part does not.
- GUI: the menu "Record texts" chooses the language of the texts the Record preview shows next
  to localisation keys, in the Record tab and in the Record XML. It lists the languages whose
  `global.ini` the open Archive holds plus "Keys only", follows the interface language until
  one is chosen, and stores the choice in the settings (`record_language`). A key without a text
  in the language is marked "Missing"; control, bidirectional and line feed characters of the
  texts are shown as escapes. A language is read, indexed and copied once per Archive; switching
  it re-renders the open Record from the DataForge already held, without reading it again.
  `LanguageTexts::owned` resolves with an `OwnedTextSet`.
- GUI: a Localisation tab next to Query. It reads the `global.ini` of every language of the open
  Archive off the UI thread behind a loading indicator and shows one row per localisation key
  and one column per language, with missing translations marked. The search matches keys and
  texts of all languages ignoring case, runs off the UI thread while typing and keeps the
  window responsive on a full table; control and bidirectional characters in texts are shown as
  escapes. The key or the text of the selected cell is copied with two actions or Ctrl+C, and
  the table scrolls with the keyboard. All texts are in the German and British English interface
  files.
- Verification of the Basetool catalog against all installed Channels: Records with the words
  `debug`, `deprecated` or `dev` in their path are left out as development leftovers, and the
  measured sizes (at most 21.1 MB against the 64 MB upload limit) are recorded in
  `docs/basetool-catalog.md`.
- An unreadable selected Record no longer aborts `p4kx catalog` and the desktop app's Basetool
  catalog: it is left out and counted as a failure, the catalog is still written, `p4kx catalog`
  lists the failed Records on stderr and under `failed` in `--json` and exits with code 1, and the
  summary window lists them.
- `p4kx dcb export --lang <language>`: Record XML, Record JSON and NDJSON with the text of every
  localisation key next to the key, in the `global.ini` of that language; a key without a text is
  marked missing, a placeholder key stays alone. Additive: without `--lang` the output is byte
  for byte the same as before, and the export report names the `language` only with it.
  `RecordXml::write_with`, `RecordJson::write_with` and `TextResolver` in `p4kx-dataforge`,
  `LanguageTexts` and `RecordExportOptions::texts` in `p4kx-extraction`; the Record JSON Schema
  allows the new `{"key", "text"|"missing"}` object. `RecordFileFormat::write_file` takes the
  texts as an extra parameter.
- `p4kx model info|glb <archive> <entry>`: static `#ivo` models (`.cgf` with their `.cgfm`
  geometry companion) shown with their nodes, LODs, meshes, materials, bones and chunks, and
  exported as one glTF 2.0 binary file (GLB) that Blender opens upright and in metres: one node
  per model node with its transform, one primitive per submesh with positions, normals and
  texture coordinates, and one material per material ID. Unknown chunks are skipped and listed.
  Both commands also take a model file on disk with its companion next to it.
- `p4kx entity audio <archive> <entity> [--json]`: the audio triggers an Entity names, each with
  the Wwise events its ATL definition posts and the soundbanks whose `HIRC` section holds them,
  matched by the events' Wwise short IDs (FNV-1). Triggers without definition, without event or
  in no soundbank are listed with the reason; ATL definitions and soundbanks that cannot be read
  are warned about. The audio index reads every ATL definition and soundbank of the Archive once,
  in parallel.
- `p4kx ui assets <archive> <entry> [--json]` and `p4kx ui extract <archive> <entry> --out
  <folder>`: list the tags, shapes, morph shapes, bitmaps, fonts and sprites of a GFx or SWF UI
  movie with their IDs and export names, and write its bitmaps as PNG and its shapes as JSON in
  twips. Compressed (zlib, LZMA) and uncompressed movies are read, unknown tags are kept and
  counted, a broken tag is reported without failing the movie (exit code 1), and GFx external
  images are decoded from the DDS textures next to the movie, on disk only inside the movie's
  `Data` folder. Also for movie files on disk.
- `p4kx socpak list <archive> <container…>` and `p4kx socpak extract <archive> <container…>`:
  socpak containers, also nested ones, are read from the Archive without extracting them first,
  listed like Archives and extracted with `--filter`, `--out`, `--conflict` and `--convert`.
  Nesting depth, Entry count, decoded size, compression ratio and overlapping Entry data are
  bounded; a corrupt container fails with exit code 3 and an error naming its Entry.
- `p4kx chf to-json <file> [--out <file>]` and `p4kx chf from-json <file> --out <file>`: CHF
  character files as JSON and back. The new `p4kx-character` crate checks magic, size and
  CRC-32C, decompresses the zstd body and reads its inner versions (7 and 8), body type, face DNA,
  item ports and material choices; an unchanged JSON gives back the original file byte for byte,
  an edited one a new file with a correct CRC-32C. Unknown versions and CRC-32C mismatches are
  errors that name them.
- `p4kx-mcp`: a read-only MCP server over stdio for AI clients, with the tools `list_channels`,
  `archive_info`, `list_entries` and `search_entries`. An Archive is named by Channel or by
  absolute path, per call or with `--archive`; listings page with a cursor, at most 1000 items a
  page. `p4kx-mcp.exe` ships in the portable ZIP and the MSI next to `p4kx.exe`.
- `p4kx-mcp` tools `read_entry_text` and `read_entry_bytes`: read an Entry in ranges from a byte
  offset, as text (CryXml converted to XML, UTF-16 by its byte order mark, otherwise UTF-8;
  binary Entries are an error) or as base64. A range is at most 256 KiB of text or 192 KiB of
  bytes; a larger `max_bytes` is clamped and the result says so, and `next_offset` continues.
  Stored Entries, encrypted or not, are read from the offset directly; compressed ones decode at
  most 1 GiB to reach it. At most four Entries are decoded whole for text at a time.
- The Audio tab of the preview: select a WEM to see its codec, channels, sample rate and
  duration, and play it without extracting, with play, pause, seek and volume. PCM, IMA ADPCM,
  PTADPCM and Wwise Vorbis sounds play; another codec, a missing audio device or a sound too long
  to hold is named instead.
- The Query tab of the window: run Queries over the DataForge of the open Archive in the
  background with progress and cancel, see an invalid Query explained with its line and column
  and underlined in the editor, sort the result table by any column, page through large results,
  and export the result as CSV or JSON exactly as `p4kx dcb query --out` writes it.
- The Record changes of a Comparison in the window: a Records tab lists every added, removed,
  moved, changed and unreadable Record with its record path and type, sorted and filtered like the
  Entries, and a selected Record shows its field changes with old and new value, links by the
  record path of their target and unreadable values with their reason.
- The Export menu of the window with **Record export…**: the Records selected in the Record tree
  or matching a record-path filter as Record JSON, one file per Record or one NDJSON file, into a
  new or an existing folder with a Conflict policy, in the background with progress, cancel and a
  summary of the Records written and those with error values. DataForge and the planned files are
  read once per open Archive and reused by every later export.
- **Export › Basetool catalog…** in the window: the Basetool catalog of the open Archive into a
  file chosen in the save dialog, in the background with progress and cancel, writing no partial
  file, and a summary of the entities and skipped Records per kind, the resolved and missing texts
  per language and the size with a warning near or over the basetool upload limit. The Record
  export and the catalog share the DataForge read once per open Archive.
- `p4kx_export::build_catalog_from`: builds the Basetool catalog from a DataForge Entry already
  read.
- `p4kx_export::write_planned_record_export` with `RecordFilePlans`, and
  `p4kx_extraction::export_planned_record_files` with `RecordFilePlan`: Record exports that plan
  the files of a DataForge's Records once; `RecordExportOptions::progress` reports every Record
  as it ends.
- `p4kx entity loadout <archive> <entity> [--json]`: the default Loadout tree of an Entity found
  by class name, GUID or record path, resolved recursively through nested and overriding
  Loadouts, XML Loadouts and port default items, with each port's sizes, types and helper; a port
  without Entity or with a dangling Reference, an unknown class name or a cycle is listed with the
  reason instead of failing the tree.
- `p4kx dcb query <archive> "<query>"`: Queries over the Records of the DataForge, defined in
  `docs/query.md`: select Records by type (subtypes included) and record-path glob, and list
  scalar fields by dot path as columns of a table, the record path first. Errors name the problem
  and mark its position in the Query, with a suggestion for a mistyped type or field name.
- `p4kx dcb query --format table|csv|json [--out <file>]`: Query results as RFC 4180 CSV with a
  header row or as JSON, an array of objects keyed by column with values typed as booleans,
  numbers and strings, written page by page to stdout or to a file; the columns of the table
  fit every row.
- `where` conditions in Queries: comparisons with `=`, `!=`, `<`, `<=`, `>`, `>=` on numbers,
  strings, GUIDs, enums by name and booleans, string matching with `contains`, `starts with` and
  `like` ignoring case, joined by `and`, `or`, `not` and parentheses. A comparison of
  incompatible types is marked as an error, and a missing value never matches.
- Paths into nested data in Queries: dot paths through inline structs and strong pointers, `[n]`
  for one array element (null when out of range), `[*]` for every element and `[TypeName]` for
  the elements of a polymorphic array of that type, e.g.
  `Components[VehicleComponentParams].crewSize`. A path through `[*]` or `[TypeName]` gives a
  list per cell, a JSON array in CSV and JSON, and a comparison over it holds when one of its
  values holds.
- `->` in Query paths follows a Reference to the Record it links to, or a weak pointer to its
  instance, in `select` and `where`, chained and behind array elements, e.g.
  `AttachDef.Manufacturer->Code` or `escorts[*]->name`. A null or dangling Reference gives null
  and is counted as unresolved in the result summary; `->` where no Reference stands is marked
  as an error.
- `p4kx catalog <archive> --out <file>`: the Basetool catalog in its new, versioned format with
  Channel, Build and completeness per kind, starting with the manufacturers; test, template,
  dummy, debug, deprecated and dev Records are left out and counted, `--filter` makes it partial,
  and a warning comes before the 64 MB upload limit. `p4kx catalog schema` prints its JSON Schema, checked in as
  `schemas/basetool-catalog.schema.json`.
- English and German names and descriptions in the Basetool catalog, resolved from the Archive's
  `global.ini` files next to their localisation keys, with `\n` as a line feed; a key or
  language the Archive lacks or an empty text leaves that text `null`, the placeholder keys
  `@LOC_EMPTY`, `@LOC_PLACEHOLDER` and `@LOC_UNINITIALIZED` give no text, and `p4kx catalog`
  counts resolved, unresolved and placeholder texts per language. A corrupt `global.ini` ends
  `p4kx catalog` with exit code 3.
- `p4kx texture info` and `p4kx texture merge`: read a DDS texture split into a header and mip
  parts, from an Archive or from files on disk, and merge it into one standard DDS file.
- `p4kx texture png`: decode a mip of a BC1, BC2, BC3 or uncompressed texture with pure-Rust
  decoders and write it as an 8-bit PNG; the largest mip by default, another with `--mip`.
- Cubemaps and alpha tails: `p4kx texture merge` merges split cubemaps and texture arrays face by
  face, `p4kx texture png` writes a cubemap as six face files or with `--cross` as one cross, an
  alpha or smoothness tail `.dds.a` is merged into the alpha channel of the PNG, and
  `p4kx texture info` reports it.
- High-range BC6H textures, signed and unsigned: `p4kx texture png` decodes them with a pure-Rust
  decoder to floating-point RGB and writes OpenEXR (`.exr`, 32-bit float RGB) instead of a PNG, so
  their range is not clipped, cubemaps as six faces or a cross; `--tone-map` writes a Reinhard
  tone-mapped 8-bit PNG instead.
- `p4kx compare <old> <new> [--entries] [--filter] [--report <file>]`: the Entries added, removed
  and changed between two Archives, from their central directories without decompressing, with
  counts per top-level directory, a JSON report with a checked-in schema, and exit code 1 when the
  Archives differ.
- The Comparison tab of the window: pick an old and a new Archive from the detected Channels, the
  Recent Archives or any file, compare their Entries, their Records or both off the UI thread with
  progress and cancel, and see both Builds, the counts per change kind and per top-level directory,
  and the sortable list of added, removed and changed Entries, filtered with the rules of the
  search. **Save report…** writes the same JSON as `p4kx compare --report` with the current
  levels and Filter, the Record level included.
- `p4kx compare --records`: the Records of both DataForges joined by GUID and listed as added,
  removed, moved or changed, with each changed scalar field and its old and new value, in the
  summary counts and the JSON report; a Record that cannot be read is listed as unreadable with
  the reason. Without `--entries` or `--records` both levels run.
- `p4kx compare --records` compares the whole Record tree: changes inside nested structs and
  arrays carry their location, e.g. `hull.plates[1].armor`, array elements added or removed at the
  end are listed as such, and changed References show the old and new target's GUID and record
  path; weak pointers compare by their location inside the Record as well.
- `p4kx compare <old> <new> --diff <entry>`: the line diff of one text, XML or CryXml Entry as a
  unified diff, or as JSON hunks with `--json`; CryXml is converted to XML first, an added or
  removed Entry is diffed against an empty side, and a binary Entry is reported as changed or not.
- `p4kx audio info` and `p4kx audio wav`: describe a Wwise WEM sound from an Archive or from disk
  and write PCM WEMs as WAV files; every Wwise codec is detected.
- `p4kx audio ogg`: rebuild a Wwise Vorbis WEM as a standard Ogg Vorbis file without re-encoding,
  from an Archive or from disk. The headers are rebuilt with the default codebooks and checked
  with a Vorbis decoder, falling back to the aoTuV codebooks; the file plays for the sound's exact
  duration. The codebook libraries come from ww2ogg under the BSD 3-Clause licence, whose notice
  ships as `THIRD-PARTY-NOTICES` in the ZIP and the MSI.
- `p4kx audio wav` and `p4kx bnk extract --format wav` decode Wwise IMA ADPCM and PTADPCM WEMs,
  mono and multi-channel, to 16-bit WAV. IMA ADPCM durations count 64 frames per 36-byte block
  per channel, as Wwise writes them, instead of 65.
- `p4kx audio wav` and `p4kx bnk extract --format wav` decode Wwise Vorbis WEMs to 16-bit WAV for
  the sound's exact duration, with the codebooks the Ogg export picks; the channels keep the
  order and speaker mask of the WEM. A sound that cannot be decoded fails with an error naming
  the WEM and the reason.
- `p4kx audio ogg` and `p4kx bnk extract --format ogg` repackage Opus WEMs (Wwise Opus, Ogg Opus
  in a WEM and Switch Opus) as standard Ogg Opus files without re-encoding, with the pre-skip,
  the channel mapping of the Wwise layout and the sound's exact duration; `p4kx audio wav` and
  `p4kx bnk extract --format wav` decode them at 48 kHz to 16-bit WAV in the speaker order of the
  WEM, with the pure-Rust decoder opus-pure under the BSD 3-Clause licence, whose notice ships in
  `THIRD-PARTY-NOTICES`. The `OggReport` of `p4kx-audio` gains a `codec` field and its
  `codebooks` becomes an `Option`, `None` for Opus; `p4kx audio ogg --json` reports `codec` and a
  `null` `codebooks` for Opus.
- `p4kx bnk list` and `p4kx bnk extract`: list the WEMs a Wwise BNK soundbank from an Archive or
  from disk embeds, with ID, codec and duration, and write them as `<id>.wem`, converted to WAV or
  rebuilt as Ogg Vorbis; the `BKHD`, `DIDX`, `DATA`, `HIRC` and `STID` sections are parsed, and a
  corrupt soundbank fails with an error naming the section.
- `p4kx chunks list` and `p4kx chunks dump`: list the chunks of a CryEngine chunk file (`.cgf`,
  `.cga`, `.skin`, `.chr`, their geometry companions, `.caf`, `.dba`) from an Archive or from disk,
  in the `#ivo` (`0x900`) or the `CrCh` (`0x746`) container, with type, type ID, version, chunk
  ID, byte order, offset and size, unknown types marked; and write one chunk's bytes to a file.
- `p4kx cryxml`: convert a CryXml file on disk (`CryXmlB`, `CryXml`, `CRY3SDK`, either byte
  order) to well-formed UTF-8 XML; text XML is written unchanged.
- Conversion during Extraction: `p4kx extract --convert` and the Conversion switch of the
  Extraction window write CryXml Entries as UTF-8 XML; text XML and every other Entry stay
  unchanged. The switch is remembered with the other Extraction settings, and the summaries and
  `--json` count the converted Entries.
- Texture Conversion during Extraction: with `p4kx extract --convert` and the Conversion switch,
  each texture is merged into one DDS file and its largest mip exported as PNG once, not once per
  part, with its alpha tail in the PNG's alpha channel, a high-range texture as OpenEXR and a
  cubemap as six face files; `--textures merge,png,raw` and `--mip <level>`, and the same choices in the Extraction
  window, pick the outputs and keep the raw parts. A texture that cannot be converted is listed
  with its reason, its raw Entries are written and the Extraction continues. The choices are
  remembered with the other Extraction settings.
- `p4kx dcb info`: the version and the number of Records, structs and enums of the DataForge
  (`Data\Game2.dcb`, falling back to `Data\Game.dcb`); versions 6 and 8 are read, so it works on
  every current Channel, and any other version is reported as not supported.
- `p4kx dcb export`: write Records as Record XML, one file per Record under its record path, with
  owned structs nested, arrays in stored order, References and weak pointers as links (GUID,
  record path, type and, inside the Record, the location) and explicit nulls. Unreadable values
  such as dangling References are reported per value while the rest of the Record is written.
- `p4kx dcb export --format json|ndjson`: write Records as Record JSON (`docs/record-json.md`,
  with a checked-in JSON Schema), one file per Record under its record path or one
  `records.ndjson` with one Record per line in record-path order, with the structure and field
  order of the Record XML, References as objects with `guid`, `path` and `type`, and errors with
  their reason. The new service crate `p4kx-export` writes the Record export in every form.
- `p4kx dcb export --combined`: every selected Record in one Record XML file in record-path
  order, named after the DataForge Entry, built in parallel and streamed to disk, byte-identical
  across runs.
- Conversion of DataForge during Extraction: with `p4kx extract --convert` or the Conversion
  switch of the Extraction window, the DataForge Entry is written as one Record XML file per
  Record below its folder instead of the `.dcb`, with progress and cancellation per Record; the
  summaries and `--json` count the Records and their error values.
- DataForge in the window: a DataForge node below the folders of the path tree shows the Records
  as folders by record path, with record folders where several Records share one. It loads in the
  background the first time it is opened, with a loading indicator, and is kept until the Archive
  is reloaded or changes on disk; an unsupported version or a load error is named on the node. The
  new DataForge tab of the preview shows the version, the numbers of Records, structs and enums and
  the first Records.
- Archive menu entries "Reload" (F5) and "Extract selected…" (Ctrl+E), with the same keyboard
  shortcuts working anywhere in the window.
- Texture groups in the window: a split texture (`.dds` with `.dds.N` parts and an Alpha tail)
  is one row with its total size, its parts one level in below it. Checking a row checks all its
  Entries, a search that matches a part shows the whole texture, sorting by size uses the total
  size, and "Selected" and "Shown" extract every part. Parts without a header stay single rows.
- The long views of the preview and the Comparison scroll with the keyboard: Tab focuses the Text,
  Hex and Image previews, the line diff, the field list of a Record change and the field list of
  the Record preview, and the arrow keys, Page Up, Page Down, Home and End scroll them like the XML
  view.
- The Extraction summary lists the Records that were not written with their record path, GUID and
  reason, as it lists failed Entries.

### Changed

- `p4kx list` folds the parts of a split texture into the row of its header: sizes of the whole
  texture, a ` (+N parts)` suffix on the path and a `parts` field in `--json`. `--parts` restores
  one row per Entry.
- `p4kx extract --filter` and `p4kx socpak extract --filter` now also take every Entry of a
  texture whose header, part or Alpha tail matches.
- The window follows the P4KX design and its desktop spec: buttons, toggles,
  tabs, tables, tree, modals and alerts, Lato in three weights, the P4K-Extractor glyph as window
  icon and header mark, an empty state with an "Open Archive" action.

## [0.1.0]

The first Release: open, browse and extract Star Citizen Archives.

### Added

- Reading Archive layouts v1 and v2: ZIP64 central directory, Store, Deflate and Zstandard
  Entries, AES-128-CBC encrypted Entries.
- Detection of the installed Channels and the Build of each, plus manually added folders and
  Archives.
- The `p4kx` command line: `list`, `search` and `extract` with JSON output and documented exit
  codes.
- The P4K-Extractor window: Channel picker, drag and drop, recent Archives, path tree, sortable
  Entry table, search with quick-filter chips, text and hex preview, and Extraction of selected,
  shown or all Entries with progress, cancellation and conflict policies.
- A notice with a reload offer when the open Archive changes on disk.
- German and British English interface, dark look.
- A portable ZIP and a per-user MSI installer.
- An optional check for a newer Release at start, off by default and switchable in the Help menu.
