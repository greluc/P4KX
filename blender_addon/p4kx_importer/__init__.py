# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Imports a P4KX Blender package (``scene.json``, GLB meshes, PNG textures) into Blender."""

from __future__ import annotations


def register() -> None:
    """Registers the import operator; Blender calls this when the add-on is enabled."""
    from . import operator

    operator.register()


def unregister() -> None:
    """Removes the import operator; Blender calls this when the add-on is disabled."""
    from . import operator

    operator.unregister()
