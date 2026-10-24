# P4KX Blender add-on

Imports a [Blender package](../docs/blender-package.md) written by `p4kx entity package` into
Blender with one action: **File > Import > P4KX Package (scene.json)**. Decisions:
[ADR-0093](../docs/adr/0093-the-blender-add-on-is-an-extension-that-reads-and-validates-a-package-without-blender.md).

**Supported Blender: 5.2 LTS** (Python 3.13), as `blender_manifest.toml` states. The glTF 2.0
importer that ships with Blender must be enabled.

## What the import does

- Builds the scene hierarchy as empties with the names and transforms of `scene.json`.
- Imports every shared mesh once and places it as a linked collection instance.
- Loads the textures with the colour space the package states and gives the materials a basic
  node setup: diffuse, normal map, specular and alpha.
- Builds the Interior of a package written with `--interior`: containers and models as empties
  and instances, lights as Blender point, spot and sun lights (mapping in
  [docs/blender-package.md](../docs/blender-package.md#interiors-and-lights)). The import option
  *Light strength* multiplies their energy, because the engine's intensity has no known unit.
- Refuses a package with a newer major version and says to update the add-on; warns about a
  newer minor version.

## Build and install

```powershell
python blender_addon/build_zip.py out
```

writes `out/p4kx_importer-<version>.zip`. In Blender: Edit > Preferences > Get Extensions >
Install from Disk. The release workflow builds the same zip and attaches it to every GitHub
Release; the manifest `version` must equal the workspace (Release) version, which `cargo xtask
addon` checks in CI.

## Tests

```powershell
cd blender_addon
python -m pip install pytest bpy==5.2.3
python -m pytest
python tests/blender_smoke.py
```

`pytest` runs without Blender. `blender_smoke.py` imports a synthetic package in headless Blender
(the `bpy` module, or `blender --background --factory-startup --python tests/blender_smoke.py`).
