# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""A basic node setup for the materials of an imported mesh: diffuse, normal, specular, alpha."""

from __future__ import annotations

from typing import TYPE_CHECKING

import bpy

from .package import Package, PackageError, Texture, resolve_file

if TYPE_CHECKING:
    from .package import MeshMaterial

_COLOUR_SPACE_NAMES = {"srgb": "sRGB", "linear": "Non-Color"}


class ImageCache:
    """Loads each texture of a package once and sets its colour space and alpha handling."""

    def __init__(self, package: Package) -> None:
        self._package = package
        self._images: dict[str, bpy.types.Image | None] = {}

    def get(self, texture_id: str) -> bpy.types.Image | None:
        """Returns the image of a texture, or ``None`` when its file cannot be loaded."""
        if texture_id not in self._images:
            self._images[texture_id] = self._load(self._package.textures[texture_id])
        return self._images[texture_id]

    def _load(self, texture: Texture) -> bpy.types.Image | None:
        try:
            path = resolve_file(self._package.root, texture.file)
            image = bpy.data.images.load(str(path), check_existing=True)
        except (PackageError, RuntimeError):
            self._package.warnings.append(f"Texture {texture.id} could not be loaded.")
            return None
        image.colorspace_settings.name = _COLOUR_SPACE_NAMES[texture.colour_space]
        image.alpha_mode = "STRAIGHT" if texture.alpha == "opacity" else "NONE"
        return image


def apply_textures(
    material: bpy.types.Material, described: MeshMaterial, package: Package, images: ImageCache
) -> None:
    """Connects the textures of ``described`` to the Principled BSDF of ``material``."""
    if material.node_tree is None:
        material.use_nodes = True
    tree = material.node_tree
    bsdf = _principled(tree)
    row = 0
    for role, texture_id in described.textures.items():
        image = images.get(texture_id)
        if image is None:
            continue
        node = tree.nodes.new("ShaderNodeTexImage")
        node.image = image
        node.location = (bsdf.location.x - 700, bsdf.location.y - 300 * row)
        row += 1
        if role == "diffuse":
            tree.links.new(node.outputs["Color"], bsdf.inputs["Base Color"])
            if package.textures[texture_id].alpha == "opacity":
                tree.links.new(node.outputs["Alpha"], bsdf.inputs["Alpha"])
        elif role == "normal":
            normal = tree.nodes.new("ShaderNodeNormalMap")
            normal.location = (bsdf.location.x - 350, node.location.y)
            tree.links.new(node.outputs["Color"], normal.inputs["Color"])
            tree.links.new(normal.outputs["Normal"], bsdf.inputs["Normal"])
        elif role == "specular":
            tree.links.new(node.outputs["Color"], bsdf.inputs["Specular IOR Level"])


def _principled(tree: bpy.types.ShaderNodeTree) -> bpy.types.Node:
    for node in tree.nodes:
        if node.type == "BSDF_PRINCIPLED":
            return node
    bsdf = tree.nodes.new("ShaderNodeBsdfPrincipled")
    output = next((n for n in tree.nodes if n.type == "OUTPUT_MATERIAL"), None)
    if output is None:
        output = tree.nodes.new("ShaderNodeOutputMaterial")
    tree.links.new(bsdf.outputs["BSDF"], output.inputs["Surface"])
    return bsdf
