# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""The File > Import operator of the add-on."""

from __future__ import annotations

from pathlib import Path

import bpy
from bpy.props import StringProperty
from bpy_extras.io_utils import ImportHelper

from .importer import import_package
from .package import PackageError, load_package


class IMPORT_SCENE_OT_p4kx_package(bpy.types.Operator, ImportHelper):
    """Import a P4KX Blender package (scene.json)"""

    bl_idname = "import_scene.p4kx_package"
    bl_label = "Import P4KX Package"
    bl_options = {"REGISTER", "UNDO"}

    filename_ext = ".json"
    filter_glob: StringProperty(default="scene.json", options={"HIDDEN"})

    def execute(self, context: bpy.types.Context) -> set[str]:
        """Loads the selected package and builds its objects, or reports why it cannot."""
        try:
            package = load_package(Path(self.filepath))
            result = import_package(context, package)
        except PackageError as error:
            self.report({"ERROR"}, str(error))
            return {"CANCELLED"}
        for warning in result.warnings:
            self.report({"WARNING"}, warning)
        self.report(
            {"INFO"},
            f"Imported {result.objects} nodes, {result.meshes} meshes, "
            f"{result.instances} instances.",
        )
        return {"FINISHED"}


def menu_import(menu: bpy.types.Menu, _context: bpy.types.Context) -> None:
    """Adds the operator to the File > Import menu."""
    menu.layout.operator(IMPORT_SCENE_OT_p4kx_package.bl_idname, text="P4KX Package (scene.json)")


def register() -> None:
    """Registers the operator and its menu entry."""
    bpy.utils.register_class(IMPORT_SCENE_OT_p4kx_package)
    bpy.types.TOPBAR_MT_file_import.append(menu_import)


def unregister() -> None:
    """Removes the menu entry and the operator."""
    bpy.types.TOPBAR_MT_file_import.remove(menu_import)
    bpy.utils.unregister_class(IMPORT_SCENE_OT_p4kx_package)
