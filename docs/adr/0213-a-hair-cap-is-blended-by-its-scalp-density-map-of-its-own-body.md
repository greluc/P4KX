# A hair cap is blended by its scalp density map of its own body

Decided for issue #739. Evidence: the LIVE Archive, `masculinedefault.chf` (hair `m_hair_31`, beard
`m_facial_hair_045`) exported with `p4kx chf glb` and imported into Blender 5.2.

## What the data says

- A hair material with `Shader="HairPBR"` and `StringGenMask="%HAIR_CAP"` (the scalp and the
  stubble) is a thin shell over the skin. It has `Diffuse="0,0,0"` and one map, in `TexSlot1`, named
  `…_scalp`: a grey density map (white where hair grows), the same for both bodies in the
  hair that was looked at. The engine blends the shell by that map.
- The GLB writer drew the cap opaque, because the `_scalp` map was no diffuse map: the shell
  z-fought with the skin and painted the face and scalp black.
- `m_hair_31.mtl` names the female map (`sc/female/textures/f_hair_31_scalp.tif`) for its cap, while
  its colour variants (`m_hair_31_brown.mtl` and the others) name the male one
  (`sc/male/textures/m_hair_31_scalp.dds`). A character that wears the base file got the wrong
  body's map.

## Decision

- A cap is drawn `BLEND`. Its base colour image is white with the red channel of the density map as
  alpha, as for the opacity map of hair cards, and the material's colour (the hair dye) is the base
  colour factor. The Blender package gives such a material the alpha mode `blend` and the `_scalp`
  file the usage `mask`, so the add-on takes the luminance of the map as alpha.
- Without its map (not in the Archive, unreadable, or no map at all) a cap is fully transparent
  and the missing file is listed as a texture fallback; it is never an opaque shell.
- The map of the wrong body is replaced. The body of a hair is the `male` or `female` folder of its
  material library; if the cap's map lies in the other body's folder, the folder and the `m_` / `f_`
  file name prefix are swapped (`sc/female/textures/f_hair_31_scalp` becomes
  `sc/male/textures/m_hair_31_scalp`). A library or map without such a folder is left as it is. A
  character wears the hair of its own body, so the library's folder is the Character's body, and the
  writer needs no further input. If the swapped file is missing, it is the one listed.

## Consequences

- The Blender package describes the materials as the file defines them, so it keeps the map the file
  names; only the GLB writer swaps the body.
