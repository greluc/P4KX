# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Builds Blender objects from a validated package: shared meshes once, nodes as instances."""

from __future__ import annotations

from dataclasses import dataclass, field

import bpy
from mathutils import Quaternion, Vector

from .materials import ImageCache, apply_textures
from .package import (
    Mesh,
    Node,
    Package,
    PackageError,
    blender_name,
    gltf_to_blender_quaternion,
    gltf_to_blender_scale,
    gltf_to_blender_vector,
    pair_materials,
    resolve_file,
)


@dataclass(slots=True)
class ImportResult:
    """What an import created, for the report to the user."""

    root: bpy.types.Collection
    objects: int = 0
    instances: int = 0
    meshes: int = 0
    warnings: list[str] = field(default_factory=list)


def import_package(context: bpy.types.Context, package: Package) -> ImportResult:
    """Imports ``package`` into the scene of ``context`` and returns what was created.

    Each mesh used by a node is imported once into a hidden template collection; every node that
    draws it becomes an empty that instances that collection. Raises ``PackageError`` when the
    glTF importer of Blender is not available.
    """
    if not hasattr(bpy.ops.import_scene, "gltf"):
        raise PackageError("Enable Blender's 'glTF 2.0 format' add-on first.")
    root = bpy.data.collections.new(blender_name(package.entity_name))
    context.scene.collection.children.link(root)
    result = ImportResult(root=root)
    templates = _import_meshes(context, package, root, result)
    created: dict[int, bpy.types.Object] = {}
    for node in package.nodes:
        template = templates.get(node.mesh) if node.mesh else None
        created[node.index] = _create_object(node, template, created, root)
        result.objects += 1
        result.instances += template is not None
    result.warnings.extend(package.warnings)
    return result


def _import_meshes(
    context: bpy.types.Context, package: Package, root: bpy.types.Collection, result: ImportResult
) -> dict[str, bpy.types.Collection]:
    wanted = {node.mesh for node in package.nodes if node.mesh}
    holder = bpy.data.collections.new(blender_name(f"{package.entity_name} meshes"))
    root.children.link(holder)
    holder_layer = _layer_collection(context.view_layer.layer_collection, holder)
    images = ImageCache(package)
    templates: dict[str, bpy.types.Collection] = {}
    for mesh_id in sorted(wanted):
        mesh = package.meshes[mesh_id]
        template = bpy.data.collections.new(mesh_id)
        holder.children.link(template)
        if _import_glb(context, package, mesh, template, images):
            templates[mesh_id] = template
            result.meshes += 1
        else:
            package.warnings.append(f"Mesh {mesh_id} could not be imported.")
    if holder_layer is not None:
        holder_layer.exclude = True
    return templates


def _import_glb(
    context: bpy.types.Context,
    package: Package,
    mesh: Mesh,
    template: bpy.types.Collection,
    images: ImageCache,
) -> bool:
    try:
        path = resolve_file(package.root, mesh.file)
    except PackageError:
        return False
    if not path.is_file():
        return False
    layer = _layer_collection(context.view_layer.layer_collection, template)
    previous = context.view_layer.active_layer_collection
    context.view_layer.active_layer_collection = layer
    before = set(bpy.data.materials)
    try:
        status = bpy.ops.import_scene.gltf(filepath=str(path))
    except RuntimeError:
        return False
    finally:
        context.view_layer.active_layer_collection = previous
    if "FINISHED" not in status:
        return False
    created = [material for material in bpy.data.materials if material not in before]
    pairs = pair_materials(
        [(material.index, material.name) for material in mesh.materials],
        [material.name for material in created],
    )
    by_name = {material.name: material for material in created}
    for described in mesh.materials:
        blender = by_name.get(pairs.get(described.index, ""))
        if blender is not None and described.textures:
            apply_textures(blender, described, package, images)
    return True


def _create_object(
    node: Node,
    template: bpy.types.Collection | None,
    created: dict[int, bpy.types.Object],
    root: bpy.types.Collection,
) -> bpy.types.Object:
    obj = bpy.data.objects.new(blender_name(node.name), None)
    obj.empty_display_type = "PLAIN_AXES"
    obj.empty_display_size = 0.5
    obj.location = Vector(gltf_to_blender_vector(node.translation))
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = Quaternion(gltf_to_blender_quaternion(node.rotation))
    obj.scale = Vector(gltf_to_blender_scale(node.scale))
    if template is not None:
        obj.instance_type = "COLLECTION"
        obj.instance_collection = template
    if node.parent is not None:
        obj.parent = created[node.parent]
    root.objects.link(obj)
    return obj


def _layer_collection(
    layer: bpy.types.LayerCollection, collection: bpy.types.Collection
) -> bpy.types.LayerCollection | None:
    if layer.collection == collection:
        return layer
    for child in layer.children:
        found = _layer_collection(child, collection)
        if found is not None:
            return found
    return None
