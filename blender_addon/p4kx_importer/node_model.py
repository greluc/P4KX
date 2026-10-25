# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""The scene nodes of a package and their validation, without Blender."""

from __future__ import annotations

import math
from collections.abc import Mapping
from dataclasses import dataclass
from typing import Any

from .light_model import Light, LightError, parse_light
from .manifest_fields import (
    PackageError,
    Quaternion,
    Vector3,
    read_array,
    read_object,
    read_text,
)

KNOWN_NODE_KINDS = frozenset({"entity", "container", "object", "light", "character"})


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


def parse_nodes(items: object, rigged: Mapping[str, bool], warnings: list[str]) -> list[Node]:
    """Returns the nodes this add-on understands; ``rigged`` says which mesh ids have a rig.

    A node of an unknown kind, a light of an unknown kind, a child of either, a child of a light
    and a ``character`` node without a rigged mesh are skipped with a warning.
    """
    nodes: list[Node] = []
    skipped: set[int] = set()
    unknown_kinds: set[str] = set()
    unknown_lights = False
    bare_characters = False
    light_children = False
    light_nodes: set[int] = set()
    for position, item in enumerate(read_array(items, "nodes")):
        where = f"nodes[{position}]"
        entry = read_object(item, where)
        parent = entry.get("parent")
        if parent is not None and (
            not isinstance(parent, int) or isinstance(parent, bool) or not 0 <= parent < position
        ):
            raise PackageError(f"{where}: parent must be the index of an earlier node.")
        kind = read_text(entry, "kind", where)
        if parent in light_nodes:
            skipped.add(position)
            light_children = True
            continue
        if kind not in KNOWN_NODE_KINDS or parent in skipped:
            skipped.add(position)
            if kind not in KNOWN_NODE_KINDS:
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
        if mesh is not None and (not isinstance(mesh, str) or mesh not in rigged):
            raise PackageError(f"{where}: mesh {mesh!r} is not a mesh of the package.")
        if kind == "character" and (mesh is None or not rigged[mesh]):
            skipped.add(position)
            bare_characters = True
            continue
        nodes.append(
            Node(
                index=position,
                kind=kind,
                name=read_text(entry, "name", where),
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
