# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Builds the Blender materials of an imported mesh from the package's descriptions (ADR-0100).

The mapping from a description to nodes is ``material_graph``; this module only creates what that
mapping describes, and the basic setup for materials without a description.
"""

from __future__ import annotations

from collections.abc import Iterable
from typing import TYPE_CHECKING, Any

import bpy

from .graph_groups import group_spec
from .graph_spec import GraphSpec, Socket
from .material_graph import (
    METAL_FROM,
    METAL_TO,
    SPECULAR_LEVEL_PER_F0,
    BuildContext,
    build_graph,
)
from .material_pairing import INDEX_PROPERTY, index_of, pair_by_index
from .package import Mesh, Package, PackageError, Texture, resolve_file

if TYPE_CHECKING:
    from .package import MeshMaterial

_COLOUR_SPACE_NAMES = {"srgb": "sRGB", "linear": "Non-Color"}
_ALPHA_MODES = {"opacity": "STRAIGHT", "smoothness": "CHANNEL_PACKED", "none": "NONE"}
_COLUMN = 280.0
_ROW = 260.0


class ImageCache:
    """Loads each texture of a package once and sets its colour space and alpha handling."""

    def __init__(self, package: Package) -> None:
        self._package = package
        self._images: dict[str, bpy.types.Image | None] = {}

    def get(self, texture_id: str) -> bpy.types.Image | None:
        """Returns the image of a texture, or ``None`` when its file cannot be loaded."""
        if texture_id not in self._images:
            texture = self._package.textures.get(texture_id)
            self._images[texture_id] = self._load(texture) if texture else None
        return self._images[texture_id]

    def _load(self, texture: Texture) -> bpy.types.Image | None:
        try:
            path = resolve_file(self._package.root, texture.file)
            image = bpy.data.images.load(str(path), check_existing=True)
        except (PackageError, RuntimeError):
            self._package.warnings.append(f"Texture {texture.id} could not be loaded.")
            return None
        image.colorspace_settings.name = _COLOUR_SPACE_NAMES[texture.colour_space]
        image.alpha_mode = _ALPHA_MODES[texture.alpha]
        return image


def apply_materials(
    created: Iterable[bpy.types.Material], package: Package, mesh: Mesh, images: ImageCache
) -> None:
    """Builds the node trees of the glTF materials one mesh import created.

    A glTF material is paired with the mesh's entry by its ``p4kx_index`` custom property, else
    by name. An entry with a description gets the node tree of that description; an entry
    without one, or a 1.0 package, gets the basic setup from the textures of the entry.
    """
    materials = {material.name: material for material in created}
    pairs = pair_by_index(
        [(entry.index, entry.name) for entry in mesh.materials],
        [(name, index_of(m.get(INDEX_PROPERTY))) for name, m in materials.items()],
    )
    context = BuildContext(
        package.material_descriptions,
        package.palettes.get(mesh.palette_id) if mesh.palette_id else None,
        {texture.id: texture.alpha for texture in package.textures.values()},
    )
    for entry in mesh.materials:
        blender = materials.get(pairs.get(entry.index, ""))
        if blender is None:
            continue
        description = package.material_descriptions.get(entry.material or "")
        if description is not None:
            spec = build_graph(description, context)
            for warning in spec.warnings:
                if warning not in package.warnings:
                    package.warnings.append(warning)
            build_tree(blender, spec, images)
        elif entry.textures:
            apply_textures(blender, entry, package, images)


def build_tree(material: bpy.types.Material, spec: GraphSpec, images: ImageCache) -> None:
    """Replaces the node tree of ``material`` with the nodes and links of ``spec``."""
    if material.node_tree is None:
        material.use_nodes = True
    tree = material.node_tree
    tree.nodes.clear()
    _populate(tree, spec, images)
    material.use_backface_culling = not spec.two_sided
    material.surface_render_method = "BLENDED" if spec.alpha_mode == "blend" else "DITHERED"
    for name, value in spec.properties.items():
        material[name] = value


def ensure_group(name: str, images: ImageCache) -> bpy.types.ShaderNodeTree:
    """Returns the shared node group ``name``, creating it on the first use in the file."""
    group = bpy.data.node_groups.get(name)
    if group is not None:
        return group
    spec = group_spec(name)
    group = bpy.data.node_groups.new(name, "ShaderNodeTree")
    for socket_name, socket_type, default in spec.inputs:
        item = group.interface.new_socket(socket_name, in_out="INPUT", socket_type=socket_type)
        item.default_value = default
    for socket_name, socket_type in spec.outputs:
        group.interface.new_socket(socket_name, in_out="OUTPUT", socket_type=socket_type)
    _populate(group, spec.graph, images)
    return group


def _populate(tree: bpy.types.NodeTree, spec: GraphSpec, images: ImageCache) -> None:
    nodes: dict[str, bpy.types.Node] = {}
    for node_spec in spec.nodes:
        node = tree.nodes.new(node_spec.kind)
        for name, value in node_spec.props.items():
            setattr(node, name, value)
        if node_spec.texture:
            node.image = images.get(node_spec.texture)
        if node_spec.group:
            node.node_tree = ensure_group(node_spec.group, images)
        nodes[node_spec.key] = node
    for node_spec in spec.nodes:
        for socket, value in node_spec.inputs.items():
            _set_default(_socket(nodes[node_spec.key].inputs, socket), value)
    for link in spec.links:
        source = _socket(nodes[link.source.key].outputs, link.source.socket)
        target = _socket(nodes[link.target.key].inputs, link.target.socket)
        tree.links.new(source, target)
    _layout(nodes, spec)


def _socket(sockets: Any, selector: Socket) -> bpy.types.NodeSocket:
    if isinstance(selector, int):
        return sockets[selector]
    for socket in sockets:
        if socket.name == selector and socket.enabled:
            return socket
    return sockets[selector]


def _set_default(socket: bpy.types.NodeSocket, value: Any) -> None:
    current = getattr(socket, "default_value", None)
    if isinstance(value, tuple) and current is not None and hasattr(current, "__len__"):
        size = len(current)
        value = (*value, 1.0)[:size] if size == 4 else value[:size]
    try:
        socket.default_value = value
    except (TypeError, ValueError):
        return


def _layout(nodes: dict[str, bpy.types.Node], spec: GraphSpec) -> None:
    depth = dict.fromkeys(nodes, 0)
    for _ in range(len(nodes)):
        changed = False
        for link in spec.links:
            wanted = depth[link.source.key] + 1
            if depth[link.target.key] < wanted:
                depth[link.target.key] = wanted
                changed = True
        if not changed:
            break
    rows: dict[int, int] = {}
    for key, node in nodes.items():
        column = depth[key]
        row = rows.get(column, 0)
        rows[column] = row + 1
        node.location = (column * _COLUMN, -row * _ROW)


def apply_textures(
    material: bpy.types.Material, described: MeshMaterial, package: Package, images: ImageCache
) -> None:
    """Connects the textures of ``described`` to the Principled BSDF of ``material``.

    This is the basic setup of format 1.0 packages and of materials without a description.
    """
    if material.node_tree is None:
        material.use_nodes = True
    tree = material.node_tree
    bsdf = _principled(tree)
    row = 0
    textured: dict[str, bpy.types.Node] = {}
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
        if role in ("diffuse", "specular"):
            textured[role] = node
    if "specular" in textured:
        _apply_specular(tree, bsdf, textured["specular"], textured.get("diffuse"))


def _apply_specular(
    tree: bpy.types.ShaderNodeTree,
    bsdf: bpy.types.Node,
    specular: bpy.types.Node,
    diffuse: bpy.types.Node | None,
) -> None:
    """Wires a specular map as the viewer does: metal share, colour mix and specular level."""
    x, y = bsdf.location.x - 350, specular.location.y
    grey = tree.nodes.new("ShaderNodeRGBToBW")
    grey.location = (x, y)
    tree.links.new(specular.outputs["Color"], grey.inputs["Color"])
    ramp = tree.nodes.new("ShaderNodeMapRange")
    ramp.location = (x + 150, y)
    ramp.data_type = "FLOAT"
    ramp.clamp = True
    ramp.inputs["From Min"].default_value = METAL_FROM
    ramp.inputs["From Max"].default_value = METAL_TO
    tree.links.new(grey.outputs["Val"], ramp.inputs["Value"])
    tree.links.new(ramp.outputs["Result"], bsdf.inputs["Metallic"])
    level = tree.nodes.new("ShaderNodeMath")
    level.location = (x + 150, y - 200)
    level.operation = "MULTIPLY"
    level.use_clamp = True
    level.inputs[1].default_value = SPECULAR_LEVEL_PER_F0
    tree.links.new(grey.outputs["Val"], level.inputs[0])
    tree.links.new(level.outputs["Value"], bsdf.inputs["Specular IOR Level"])
    if diffuse is not None:
        mix = tree.nodes.new("ShaderNodeMix")
        mix.location = (x + 300, y)
        mix.data_type = "RGBA"
        tree.links.new(diffuse.outputs["Color"], _enabled(mix.inputs, "A"))
        tree.links.new(specular.outputs["Color"], _enabled(mix.inputs, "B"))
        tree.links.new(ramp.outputs["Result"], _enabled(mix.inputs, "Factor"))
        tree.links.new(_enabled(mix.outputs, "Result"), bsdf.inputs["Base Color"])


def _enabled(sockets: Any, name: str) -> bpy.types.NodeSocket:
    return next(s for s in sockets if s.name == name and s.enabled)


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
