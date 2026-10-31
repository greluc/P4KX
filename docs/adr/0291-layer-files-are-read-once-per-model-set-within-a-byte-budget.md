# Layer files are read once per model set, within a byte budget

Decided for issue #561 (follow-ups of #547). It amends ADR-0097 (layered materials), whose reading
rule fetched at most 256 layer files per library and kept each parsed library for the load.

## Decision

**Surfaces, not libraries.** `MaterialStatus::load` keeps, per layer file, only the surfaces
(`LayerSurface`: diffuse, specular, texture slots) of its sub-materials; the parsed file is dropped
once they are taken. A `Submtl` is picked by one rule, `MaterialLibrary::layer_position`, which the
reader and the Blender package share.

**Bounded work.** One load fetches at most `MAX_LAYER_FILES` (256) files and at most
`MAX_LAYER_BYTES` (64 MiB) of them; a layer whose file would pass either is unresolved and listed
in `MaterialLibrary::unresolved_layers`, as a missing file is. The limits of the `.mtl` reader of
#158 still apply to each file.

**One cache.** A `LayerCache` holds the surfaces of the layer files read so far; a caller that loads
several libraries of one model set passes the same cache (`MaterialStatus::load_cached`), so a file
many libraries layer in is fetched and parsed once. `ArchiveModels` owns one for the Assembly, the
GUI exports and the Blender package. A cache keeps at most 1,024 files and starts over when full;
a file that was skipped for the budget is not cached, so the next load tries it again.

**Only drawing resolves layers.** `MaterialStatus::load_plain` and `Model::read_plain_materials`
read the library without the layer files and the diffuse maps; `p4kx model info` and the MCP model
tools use them. The default reads (`load`, `Model::read*`, `ModelSource::from_*`) resolve layers,
as every drawing caller needs.

## Consequences

The Blender package still reads each layer file as a full material through the model provider,
because it describes the layer's whole material in the manifest, which a `LayerSurface` does not
hold; only the `Submtl` pick is shared.
