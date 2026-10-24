# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Imports a synthetic package in headless Blender and checks objects, instancing and materials.

Run with ``blender --background --factory-startup --python blender_smoke.py`` or, with the
``bpy`` wheel, ``python blender_smoke.py``.
"""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

import bpy
import addon_utils

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent)]

import p4kx_importer  # noqa: E402
import synthetic  # noqa: E402
from p4kx_importer import importer, package  # noqa: E402


def check(condition: bool, message: str) -> None:
    """Raises ``SystemExit`` with ``message`` when ``condition`` is false."""
    if not condition:
        raise SystemExit(f"smoke test failed: {message}")


def main() -> None:
    """Runs the import and the checks."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    addon_utils.enable("io_scene_gltf2", default_set=False)
    with tempfile.TemporaryDirectory() as folder:
        scene = synthetic.write_package(Path(folder))
        loaded = package.load_package(scene)
        result = importer.import_package(bpy.context, loaded)
        check(result.objects == 4, f"node empties: {result.objects}")
        check(result.instances == 3, f"instances: {result.instances}")
        check(result.meshes == 2, f"meshes: {result.meshes}")

        names = {node.name for node in loaded.nodes}
        nodes = [o for o in bpy.data.objects if o.name in names]
        check(len(nodes) == 4, f"named node objects: {len(nodes)}")
        gun_a = bpy.data.objects["hardpoint_a/Gun"]
        gun_b = bpy.data.objects["hardpoint_b/Gun"]
        box = bpy.data.objects["hardpoint_c/Box"]
        check(gun_a.instance_collection is gun_b.instance_collection, "guns share a collection")
        check(gun_a.instance_collection is not box.instance_collection, "box has its own")
        check(gun_a.parent is bpy.data.objects["TEST_Ship"], "hierarchy kept")
        check(tuple(round(v, 4) for v in gun_a.location) == (1.0, -3.0, 2.0), "axes converted")

        bpy.context.view_layer.update()
        depsgraph = bpy.context.evaluated_depsgraph_get()
        placed = [
            inst.object.matrix_world @ inst.object.data.vertices[1].co
            for inst in depsgraph.object_instances
            if inst.is_instance and inst.parent.original is gun_a and inst.object.type == "MESH"
        ]
        check(len(placed) == 1, f"instanced gun meshes: {len(placed)}")
        expected = (3.0, 2.0, 2.0)
        check(
            all(abs(a - b) < 1e-4 for a, b in zip(placed[0], expected)),
            f"world position {tuple(placed[0])}, expected {expected}",
        )

        meshes = [o for o in bpy.data.objects if o.type == "MESH"]
        listing = [(o.name, o.type) for o in bpy.data.objects]
        check(len(meshes) == 2, f"mesh objects imported once each: {listing}")

        material = bpy.data.materials["Gun_mat"]
        kinds = {n.type for n in material.node_tree.nodes}
        check({"TEX_IMAGE", "NORMAL_MAP", "BSDF_PRINCIPLED"} <= kinds, f"nodes: {kinds}")
        images = [n.image for n in material.node_tree.nodes if n.type == "TEX_IMAGE"]
        spaces = {image.colorspace_settings.name for image in images}
        check(spaces == {"sRGB", "Non-Color"}, f"colour spaces: {spaces}")
        check(not result.warnings, f"warnings: {result.warnings}")

        bpy.ops.wm.read_factory_settings(use_empty=True)
        addon_utils.enable("io_scene_gltf2", default_set=False)
        p4kx_importer.register()
        status = bpy.ops.import_scene.p4kx_package(filepath=str(scene))
        check(status == {"FINISHED"}, f"operator: {status}")
        check(len([o for o in bpy.data.objects if o.type == "MESH"]) == 2, "operator import")

        newer = synthetic.manifest(version="2.0")
        newer_scene = synthetic.write_package(Path(folder) / "newer", newer)
        try:
            bpy.ops.import_scene.p4kx_package(filepath=str(newer_scene))
        except RuntimeError as error:
            check("Update the P4KX Blender add-on" in str(error), f"message: {error}")
        else:
            check(False, "a newer package must be refused")
        p4kx_importer.unregister()

    print("smoke test passed")


main()
