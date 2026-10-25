# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Imports packages of a real Archive in headless Blender and prints one JSON line per package.

Run with ``python import_real_package.py <package folder>...`` in an environment with the ``bpy``
wheel, or with ``blender --background --factory-startup --python import_real_package.py -- <folder>...``.
The exit status is 1 when a package fails to import; warnings are part of the printed result.
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

import bpy
import addon_utils

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE.parent)]

from p4kx_importer import importer, package  # noqa: E402


def folders() -> list[Path]:
    """Returns the package folders named on the command line."""
    arguments = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else sys.argv[1:]
    return [Path(argument) for argument in arguments]


def import_one(folder: Path) -> dict[str, object]:
    """Imports the package in ``folder`` into an empty scene and returns what it created."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    addon_utils.enable("io_scene_gltf2", default_set=False)
    started = time.perf_counter()
    loaded = package.load_package(folder / "scene.json")
    result = importer.import_package(bpy.context, loaded)
    elapsed = time.perf_counter() - started
    return {
        "package": folder.name,
        "seconds": round(elapsed, 2),
        "objects": len(bpy.data.objects),
        "node_objects": result.objects,
        "instances": result.instances,
        "lights": result.lights,
        "meshes": result.meshes,
        "mesh_data": len(bpy.data.meshes),
        "collections": len(bpy.data.collections),
        "armatures": result.armatures,
        "shape_keys": result.shape_keys,
        "actions": result.actions,
        "materials": len(bpy.data.materials),
        "node_groups": len(bpy.data.node_groups),
        "images": len(bpy.data.images),
        "load_warnings": list(loaded.warnings),
        "warnings": result.warnings,
    }


def main() -> int:
    """Imports every package, prints the results and returns the exit status."""
    status = 0
    for folder in folders():
        try:
            report = import_one(folder)
        except Exception as error:
            report = {"package": folder.name, "error": f"{type(error).__name__}: {error}"}
            status = 1
        print(json.dumps(report))
    return status


if __name__ == "__main__":
    sys.exit(main())
