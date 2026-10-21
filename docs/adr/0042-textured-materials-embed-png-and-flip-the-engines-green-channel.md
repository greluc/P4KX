# Textured materials embed PNG images and flip the engine's green channel

Decided for issue #105. Evidence: docs/model-materials.md (textured materials).

`p4kx model glb --materials textures` turns the diffuse, normal and specular maps of the materials
into glTF's base colour, normal and metallic-roughness textures, embedded as PNG in the binary
chunk of the GLB.

- **Green channel.** The engine's normal maps have green pointing down the image and its
  bitangent runs along +V, down the image; glTF's normal maps and the bitangent viewers derive
  point up it. The writer flips the green channel of every normal map (`255 − G`) and negates the
  handedness `w` of every `TANGENT`, so both kinds of viewer, with and without the attribute,
  agree. Evidence: the stored bitangent along +V in 99.2 % of 4.06 million triangles, and the
  integrability of 1,450 normal maps, 92 % of which have green down, checked on synthetic height
  fields. The blue channel is the one the BC5 decoder reconstructs (`√(1−x²−y²)`); BC5 stores X
  and Y only. `Mesh::tangents` keeps the engine's `w`; ADR-0028 left the question open.
- **Roughness and metalness.** The smoothness is the alpha tail of the `_ddna` map; roughness is
  `1 − smoothness`, linear, the scale of the plain `1 − Shininess / 255`, not StarBreaker's
  `√(1−s)` for the texture next to `1−s` for the scalar. The material's `Shininess` does not
  multiply the texture. The engine has no metalness map: metalness is the linear luminance of the
  tinted specular map from 0.15 to 0.5, and a metal's colour is its specular colour, so with a
  specular map the base colour image is the tinted diffuse colour moved towards the tinted
  specular colour by that metalness, and its factor is white. A metal without that image, because
  it has no specular map or the map failed to load, keeps the plain metal of ADR-0032: its
  specular colour is the base colour factor, its metalness 1, and a diffuse map multiplies the
  factor untouched (a dark diffuse colour would otherwise export a black dielectric). Only
  what the heuristic of the plain colours takes for a metal is treated so; any other material
  with a diffuse map and no specular map is a dielectric whose diffuse colour is the factor.
- **Roles by slot and name.** `TexSlot1` `_diff`, `TexSlot2` or `TexSlot3` `_ddna`/`_ddn`,
  `TexSlot4` `_spec`; the second layer, detail, displacement and tint maps are #127. The `.tif`
  name of the `.mtl` file is the `.dds` Entry; diffuse and specular bytes are sRGB although the
  format says `UNORM`.
- **PNG in the GLB, once.** One file is easier to move than a GLB with images next to it, and an
  image is made once per key (its sources and the colours that go into it) however many
  materials use it, which is the sharing #126 extends across models. At most 256 images and
  2^29 pixels per file and 2^26 pixels per texture, so that a ship cannot exhaust memory or the
  4 GiB of a GLB; what does not fit is left out and listed.
- **A texture is a status, not an error.** A texture that is missing, has an unsafe path, cannot
  be decoded or does not fit leaves the material its plain colour and is listed in the summary
  (`GlbSummary::texture_fallbacks`, `p4kx model glb` in text and `--json`), like a missing
  material library (ADR-0032). The names come from the Archive and are sanitised before they
  reach a terminal.
- **The writer takes a loader.** `p4kx-gltf` knows no Archive: `write_glb_with` takes a
  `TextureLoader` that returns a decoded mip, the extraction service implements it over the path
  index (`ModelSource::write_glb_textured`), and a model on disk has none. The path a loader gets
  is already normalised and free of `..`, absolute and drive-qualified forms.

## Considered Options

- Images next to the GLB: a moved file loses its textures; rejected.
- Leave the green channel and the tangents as the engine has them: right in a viewer that takes
  `TANGENT`, inverted in Blender, which derives its own; rejected for it.
- Flip only the texture or only `w`: right for one kind of viewer, wrong for the other; rejected.
- Bake the colours into every base colour image: no sharing between materials with different
  tints; done only where a specular map needs them (no specular map: the diffuse colour stays a
  factor).
- `KHR_materials_specular` or the deprecated specular-glossiness extension for the specular map:
  fewer viewers read them; the metalness blend works everywhere.
