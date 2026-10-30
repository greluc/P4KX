# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Takes the vertex-colour tint out of glass whose meshes carry no vertex colours (ADR-0253).

A vertex colour node reads black without its attribute, so tinted glass on a mesh imported
without colours would turn black instead of keeping its plain colour.
"""

from __future__ import annotations

from collections.abc import Iterable

import bpy

from .material_surface import GLTF_COLOUR_ATTRIBUTE

_WHITE = (1.0, 1.0, 1.0, 1.0)


def drop_tint_without_colours(
    materials: Iterable[bpy.types.Material], objects: Iterable[bpy.types.Object]
) -> None:
    """Unlinks the vertex colours from glass that no mesh in ``objects`` gives colours to."""
    coloured = {
        material
        for obj in objects
        if obj.type == "MESH" and obj.data.color_attributes.get(GLTF_COLOUR_ATTRIBUTE) is not None
        for material in obj.data.materials
    }
    for material in materials:
        if material not in coloured and material.get("p4kx_family") == "glass":
            _plain_colour(material)


def _plain_colour(material: bpy.types.Material) -> None:
    tree = material.node_tree
    if tree is None:
        return
    for node in [n for n in tree.nodes if n.bl_idname == "ShaderNodeVertexColor"]:
        for link in list(node.outputs["Color"].links):
            target = link.to_socket
            if link.to_node.bl_idname == "ShaderNodeMix":
                mix = link.to_node
                colours = [s for s in mix.inputs if s.enabled and s.type == "RGBA"]
                other = next(s for s in colours if s != target)
                for out in list(mix.outputs["Result"].links):
                    out.to_socket.default_value = tuple(other.default_value)
                    tree.links.remove(out)
                tree.nodes.remove(mix)
            else:
                target.default_value = _WHITE
        tree.nodes.remove(node)
