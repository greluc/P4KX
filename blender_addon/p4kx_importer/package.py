# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Reading and validating a P4KX Blender package (``scene.json``) without Blender.

Everything in a package is untrusted input: this module checks the format name, the version,
the fields it relies on and every file reference, and never executes anything from the package.
"""

from __future__ import annotations

import json
import math
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .light_model import Light, LightError, parse_light
from .material_model import MaterialDesc, Palette, parse_materials, parse_palettes
from .rig_model import Rig, RigError, parse_rig

FORMAT_NAME = "p4kx-blender-package"
SUPPORTED_MAJOR = 1
SUPPORTED_MINOR = 3
MANIFEST_NAME = "scene.json"
MAX_MANIFEST_BYTES = 64 * 1024 * 1024
MAX_ITEMS = 200_000
MAX_GLB_JSON_BYTES = 16 * 1024 * 1024
BLENDER_NAME_BYTES = 63

_VERSION = re.compile(r"^([0-9]+)\.([0-9]+)$")
_MESH_ID = re.compile(r"^m[0-9]{4,}$")
_TEXTURE_ID = re.compile(r"^t[0-9]{4,}$")
_MESH_FILE = re.compile(r"^meshes/m[0-9]{4,}\.glb$")
_TEXTURE_FILE = re.compile(r"^textures/t[0-9]{4,}\.png$")
_ROLES = ("diffuse", "normal", "specular")
_KNOWN_NODE_KINDS = frozenset({"entity", "container", "object", "light", "character"})
_COLOUR_SPACES = frozenset({"srgb", "linear"})
_ALPHA_MEANINGS = frozenset({"none", "opacity", "smoothness"})

Vector3 = tuple[float, float, float]
Quaternion = tuple[float, float, float, float]


class PackageError(Exception):
    """The package cannot be read: it is malformed or not a P4KX package."""


class PackageTooNewError(PackageError):
    """The package has a higher major version than this add-on reads."""


@dataclass(frozen=True, slots=True)
class MeshMaterial:
    """A material of a mesh file and the texture ids written for it."""

    index: int
    name: str
    textures: dict[str, str]
    material: str | None = None


@dataclass(frozen=True, slots=True)
class Mesh:
    """One GLB file of the package."""

    id: str
    file: str
    materials: tuple[MeshMaterial, ...]
    palette_id: str | None = None
    rig: Rig | None = None


@dataclass(frozen=True, slots=True)
class Texture:
    """One PNG file of the package."""

    id: str
    file: str
    colour_space: str
    alpha: str


@dataclass(frozen=True, slots=True)
class Node:
    """One node of the scene hierarchy, with its transform in glTF axes."""

    index: int
    kind: str
    name: str
    parent: int | None
    mesh: str | None
    translation: Vector3
    rotation: Quaternion
    scale: Vector3
    light: Light | None = None


@dataclass(slots=True)
class Package:
    """A validated package: the nodes this add-on understands and the files they use."""

    root: Path
    version: tuple[int, int]
    entity_name: str
    nodes: list[Node]
    meshes: dict[str, Mesh]
    textures: dict[str, Texture]
    warnings: list[str] = field(default_factory=list)
    material_descriptions: dict[str, MaterialDesc] = field(default_factory=dict)
    palettes: dict[str, Palette] = field(default_factory=dict)


def load_package(location: Path) -> Package:
    """Reads and validates the package at ``location``, a package folder or its ``scene.json``."""
    manifest_path = location / MANIFEST_NAME if location.is_dir() else location
    if manifest_path.name != MANIFEST_NAME:
        raise PackageError(f"Select the {MANIFEST_NAME} of a package, not {manifest_path.name}.")
    try:
        size = manifest_path.stat().st_size
    except OSError as error:
        raise PackageError(f"Cannot read {manifest_path}: {error.strerror}.") from error
    if size > MAX_MANIFEST_BYTES:
        raise PackageError(f"{MANIFEST_NAME} is larger than {MAX_MANIFEST_BYTES} bytes.")
    try:
        data = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, RecursionError, ValueError) as error:
        raise PackageError(f"{MANIFEST_NAME} is not valid JSON: {error}") from error
    return parse_manifest(data, manifest_path.parent)


def parse_manifest(data: object, root: Path) -> Package:
    """Validates a decoded ``scene.json`` and returns the package, or raises ``PackageError``."""
    manifest = _object(data, MANIFEST_NAME)
    if manifest.get("format") != FORMAT_NAME:
        raise PackageError(f"Not a P4KX Blender package: format is not {FORMAT_NAME}.")
    version, warnings = _check_version(manifest.get("version"))
    _check_coordinates(_object(manifest.get("coordinates"), "coordinates"))
    textures = _parse_textures(_array(manifest.get("textures"), "textures"), warnings)
    meshes = _parse_meshes(_array(manifest.get("meshes"), "meshes"), textures)
    nodes = _parse_nodes(_array(manifest.get("nodes"), "nodes"), meshes, warnings)
    return Package(
        root=root,
        version=version,
        entity_name=_entity_name(manifest),
        nodes=nodes,
        meshes=meshes,
        textures=textures,
        warnings=warnings,
        material_descriptions=parse_materials(manifest.get("materials"), warnings),
        palettes=parse_palettes(manifest.get("palettes"), warnings),
    )


def resolve_file(root: Path, relative: str) -> Path:
    """Returns the absolute path of a package file, refusing anything outside ``root``."""
    base = root.resolve()
    path = (base / relative).resolve()
    if not path.is_relative_to(base):
        raise PackageError(f"File {relative!r} lies outside the package folder.")
    return path


def check_glb(path: Path) -> None:
    """Refuses a GLB whose header is wrong or whose JSON names an external ``uri``.

    Only the 12-byte header and the JSON chunk (at most ``MAX_GLB_JSON_BYTES``) are read. The
    GLBs of a package embed everything, so a ``uri`` on a buffer or image is never legitimate.
    """
    try:
        with path.open("rb") as stream:
            header = stream.read(20)
            if len(header) < 20 or header[:4] != b"glTF":
                raise PackageError(f"{path.name} is not a GLB file.")
            length = int.from_bytes(header[12:16], "little")
            if header[16:20] != b"JSON" or length > MAX_GLB_JSON_BYTES:
                raise PackageError(f"{path.name} has no usable JSON chunk.")
            raw = stream.read(length)
    except OSError as error:
        raise PackageError(f"Cannot read {path.name}: {error.strerror}.") from error
    if len(raw) != length:
        raise PackageError(f"{path.name} is truncated.")
    try:
        document = json.loads(raw)
    except (RecursionError, ValueError) as error:
        raise PackageError(f"{path.name} has invalid JSON.") from error
    if not isinstance(document, dict):
        raise PackageError(f"{path.name} has invalid JSON.")
    for key in ("buffers", "images"):
        entries = document.get(key)
        if isinstance(entries, list) and any(
            isinstance(entry, dict) and "uri" in entry for entry in entries
        ):
            raise PackageError(f"{path.name} refers to an external file in {key}.")


def blender_name(name: str, limit: int = BLENDER_NAME_BYTES) -> str:
    """Shortens ``name`` to at most ``limit`` UTF-8 bytes without splitting a character."""
    encoded = name.encode("utf-8")
    if len(encoded) <= limit:
        return name
    return encoded[:limit].decode("utf-8", errors="ignore")


def gltf_to_blender_vector(vector: Vector3) -> Vector3:
    """Converts a glTF (Y up) vector to Blender's Z-up axes."""
    x, y, z = vector
    return (x, -z, y)


def gltf_to_blender_scale(scale: Vector3) -> Vector3:
    """Converts a glTF scale to Blender's axes: the Y and Z factors swap."""
    x, y, z = scale
    return (x, z, y)


def gltf_to_blender_quaternion(rotation: Quaternion) -> Quaternion:
    """Converts a glTF quaternion ``[x, y, z, w]`` to Blender's ``(w, x, y, z)`` in Z-up axes."""
    x, y, z, w = rotation
    return (w, x, -z, y)


def pair_materials(manifest_names: list[tuple[int, str]], blender_names: list[str]) -> dict[int, str]:
    """Pairs manifest materials with the Blender materials one GLB import created.

    Blender suffixes a repeated name with ``.001``; the pairing strips that suffix when the bare
    name is one the manifest knows, then matches equal names in order of index and of name.
    """
    manifest_names = [(index, blender_name(name)) for index, name in manifest_names]
    known = {name for _, name in manifest_names}
    groups: dict[str, list[str]] = {}
    for blender_name_ in sorted(blender_names):
        stripped = re.sub(r"\.[0-9]{3}$", "", blender_name_)
        base = stripped if stripped in known and blender_name_ not in known else blender_name_
        groups.setdefault(base, []).append(blender_name_)
    pairs: dict[int, str] = {}
    for index, name in sorted(manifest_names):
        candidates = groups.get(name)
        if candidates:
            pairs[index] = candidates.pop(0)
    return pairs


def _check_version(value: object) -> tuple[tuple[int, int], list[str]]:
    match = _VERSION.fullmatch(value) if isinstance(value, str) else None
    if match is None:
        raise PackageError("The package version is missing or not <major>.<minor>.")
    major, minor = int(match.group(1)), int(match.group(2))
    if major > SUPPORTED_MAJOR:
        raise PackageTooNewError(
            f"This package has format version {major}.{minor}, but this add-on reads "
            f"version {SUPPORTED_MAJOR}.x. Update the P4KX Blender add-on to the one that "
            "ships with your P4KX release."
        )
    if major < SUPPORTED_MAJOR:
        raise PackageError(f"Package format version {major}.{minor} is not supported.")
    warnings: list[str] = []
    if minor > SUPPORTED_MINOR:
        warnings.append(
            f"The package has format version {major}.{minor}, newer than the "
            f"{SUPPORTED_MAJOR}.{SUPPORTED_MINOR} this add-on reads: parts of it are ignored. "
            "Update the P4KX Blender add-on."
        )
    return (major, minor), warnings


def _check_coordinates(coordinates: dict[str, Any]) -> None:
    if coordinates.get("basis") != "gltf_y_up":
        raise PackageError("The package uses a coordinate basis this add-on does not know.")
    if coordinates.get("unit") != "metre":
        raise PackageError("The package uses a length unit this add-on does not know.")


def _entity_name(manifest: dict[str, Any]) -> str:
    source = manifest.get("source")
    entity = source.get("entity") if isinstance(source, dict) else None
    name = entity.get("class_name") if isinstance(entity, dict) else None
    return name if isinstance(name, str) and name else "P4KX package"


def _parse_textures(items: list[Any], warnings: list[str]) -> dict[str, Texture]:
    textures: dict[str, Texture] = {}
    for position, item in enumerate(items):
        where = f"textures[{position}]"
        entry = _object(item, where)
        texture_id = _text(entry, "id", where)
        if not _TEXTURE_ID.fullmatch(texture_id):
            raise PackageError(f"{where}: id {texture_id!r} is not a texture id.")
        if texture_id in textures:
            raise PackageError(f"{where}: duplicate texture id {texture_id}.")
        if entry.get("format") != "png":
            warnings.append(f"Texture {texture_id} has a format this add-on does not read.")
            continue
        file = _text(entry, "file", where)
        if not _TEXTURE_FILE.fullmatch(file):
            raise PackageError(f"{where}: file {file!r} is not a package texture path.")
        colour_space = _text(entry, "colour_space", where)
        alpha = _text(entry, "alpha", where)
        if colour_space not in _COLOUR_SPACES or alpha not in _ALPHA_MEANINGS:
            warnings.append(f"Texture {texture_id} has a colour space or alpha this add-on does not know.")
            continue
        textures[texture_id] = Texture(texture_id, file, colour_space, alpha)
    return textures


def _parse_meshes(items: list[Any], textures: dict[str, Texture]) -> dict[str, Mesh]:
    meshes: dict[str, Mesh] = {}
    for position, item in enumerate(items):
        where = f"meshes[{position}]"
        entry = _object(item, where)
        mesh_id = _text(entry, "id", where)
        if not _MESH_ID.fullmatch(mesh_id):
            raise PackageError(f"{where}: id {mesh_id!r} is not a mesh id.")
        if mesh_id in meshes:
            raise PackageError(f"{where}: duplicate mesh id {mesh_id}.")
        file = _text(entry, "file", where)
        if not _MESH_FILE.fullmatch(file):
            raise PackageError(f"{where}: file {file!r} is not a package mesh path.")
        materials = tuple(
            _parse_material(material, f"{where}.materials[{slot}]", textures)
            for slot, material in enumerate(_array(entry.get("materials"), f"{where}.materials"))
        )
        palette_id = entry.get("palette_id")
        meshes[mesh_id] = Mesh(
            mesh_id,
            file,
            materials,
            palette_id if isinstance(palette_id, str) else None,
            _rig(entry.get("rig"), f"{where}.rig"),
        )
    return meshes


def _parse_material(item: Any, where: str, textures: dict[str, Texture]) -> MeshMaterial:
    entry = _object(item, where)
    index = entry.get("index")
    if not isinstance(index, int) or isinstance(index, bool) or index < 0:
        raise PackageError(f"{where}: index is not a non-negative integer.")
    named = _object(entry.get("textures"), f"{where}.textures")
    used = {
        role: texture_id
        for role in _ROLES
        if isinstance(texture_id := named.get(role), str) and texture_id in textures
    }
    description = entry.get("material")
    return MeshMaterial(
        index,
        _text(entry, "name", where),
        used,
        description if isinstance(description, str) else None,
    )


def _parse_nodes(items: list[Any], meshes: dict[str, Mesh], warnings: list[str]) -> list[Node]:
    nodes: list[Node] = []
    skipped: set[int] = set()
    unknown_kinds: set[str] = set()
    unknown_lights = False
    bare_characters = False
    light_children = False
    light_nodes: set[int] = set()
    for position, item in enumerate(items):
        where = f"nodes[{position}]"
        entry = _object(item, where)
        parent = entry.get("parent")
        if parent is not None and (
            not isinstance(parent, int) or isinstance(parent, bool) or not 0 <= parent < position
        ):
            raise PackageError(f"{where}: parent must be the index of an earlier node.")
        kind = _text(entry, "kind", where)
        if parent in light_nodes:
            skipped.add(position)
            light_children = True
            continue
        if kind not in _KNOWN_NODE_KINDS or parent in skipped:
            skipped.add(position)
            if kind not in _KNOWN_NODE_KINDS:
                unknown_kinds.add(kind)
            continue
        light = None
        if kind == "light":
            light = _light(entry.get("light"), f"{where}.light")
            if light is None:
                skipped.add(position)
                unknown_lights = True
                continue
            light_nodes.add(position)
        mesh = None if kind == "light" else entry.get("mesh")
        if mesh is not None and (not isinstance(mesh, str) or mesh not in meshes):
            raise PackageError(f"{where}: mesh {mesh!r} is not a mesh of the package.")
        if kind == "character" and (mesh is None or meshes[mesh].rig is None):
            skipped.add(position)
            bare_characters = True
            continue
        nodes.append(
            Node(
                index=position,
                kind=kind,
                name=_text(entry, "name", where),
                parent=parent,
                mesh=mesh,
                translation=_floats(entry, "translation", 3, where),
                rotation=_rotation(entry, where),
                scale=_floats(entry, "scale", 3, where),
                light=light,
            )
        )
    if unknown_kinds:
        warnings.append(f"Nodes of kind {', '.join(sorted(unknown_kinds))} are skipped.")
    if unknown_lights:
        warnings.append("Lights of a kind this add-on does not know are skipped.")
    if light_children:
        warnings.append("Children of light nodes are skipped.")
    if bare_characters:
        warnings.append("Character nodes without a rigged mesh are skipped.")
    return nodes


def _rig(value: object, where: str) -> Rig | None:
    if value is None:
        return None
    try:
        return parse_rig(value, where)
    except RigError as error:
        raise PackageError(str(error)) from error


def _light(value: object, where: str) -> Light | None:
    try:
        return parse_light(value, where)
    except LightError as error:
        raise PackageError(str(error)) from error


def _rotation(entry: dict[str, Any], where: str) -> Quaternion:
    x, y, z, w = _floats(entry, "rotation", 4, where)
    length = math.sqrt(x * x + y * y + z * z + w * w)
    if length < 1e-6:
        raise PackageError(f"{where}: rotation is the zero quaternion.")
    return (x / length, y / length, z / length, w / length)


def _floats(entry: dict[str, Any], key: str, count: int, where: str) -> tuple[float, ...]:
    values = entry.get(key)
    message = f"{where}: {key} must be {count} finite numbers."
    if not isinstance(values, list) or len(values) != count:
        raise PackageError(message)
    converted: list[float] = []
    for value in values:
        if not isinstance(value, (int, float)) or isinstance(value, bool):
            raise PackageError(message)
        try:
            number = float(value)
        except OverflowError as error:
            raise PackageError(message) from error
        if not math.isfinite(number):
            raise PackageError(message)
        converted.append(number)
    return tuple(converted)


def _object(value: object, where: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise PackageError(f"{where} must be an object.")
    return value


def _array(value: object, where: str) -> list[Any]:
    if not isinstance(value, list):
        raise PackageError(f"{where} must be an array.")
    if len(value) > (limit := MAX_ITEMS):
        raise PackageError(f"{where} has more than {limit} entries.")
    return value


def _text(entry: dict[str, Any], key: str, where: str) -> str:
    value = entry.get(key)
    if not isinstance(value, str):
        raise PackageError(f"{where}: {key} must be a string.")
    return value
