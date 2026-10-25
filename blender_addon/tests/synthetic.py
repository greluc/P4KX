# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Writes a small synthetic P4KX Blender package for tests."""

from __future__ import annotations

import json
import struct
import zlib
from pathlib import Path
from typing import Any

TRIANGLE = struct.pack("<9f", 0, 0, 0, 1, 0, 0, 0, 1, 0)


def manifest(**overrides: Any) -> dict[str, Any]:
    """Returns a valid format 1.0 manifest: a root, two nodes of mesh m0001 and one of m0002."""
    texture = {
        "id": "t0001",
        "file": "textures/t0001.png",
        "source": "Textures/a_diff.dds",
        "format": "png",
        "bit_depth": 8,
        "colour_space": "srgb",
        "alpha": "opacity",
        "width": 1,
        "height": 1,
    }
    normal = {**texture, "id": "t0002", "file": "textures/t0002.png", "colour_space": "linear"}
    normal["alpha"] = "smoothness"
    identity = {
        "kind": "entity",
        "translation": [0.0, 0.0, 0.0],
        "rotation": [0.0, 0.0, 0.0, 1.0],
        "scale": [1.0, 1.0, 1.0],
        "path": "",
    }
    value: dict[str, Any] = {
        "format": "p4kx-blender-package",
        "version": "1.0",
        "generator": {"name": "p4kx", "version": "0.3.0"},
        "source": {"entity": {"class_name": "TEST_Ship", "guid": "g"}},
        "coordinates": {"basis": "gltf_y_up", "unit": "metre"},
        "options": {
            "lod": 0,
            "texture_mip": 0,
            "vertex_colours": True,
            "exclude": [],
            "replace": [],
        },
        "nodes": [
            {**identity, "name": "TEST_Ship", "rotation": [0.0, 0.70710678, 0.0, 0.70710678]},
            {
                **identity,
                "name": "hardpoint_a/Gun",
                "parent": 0,
                "mesh": "m0001",
                "translation": [1.0, 2.0, 3.0],
            },
            {
                **identity,
                "name": "hardpoint_b/Gun",
                "parent": 0,
                "mesh": "m0001",
                "translation": [-1.0, 2.0, 3.0],
            },
            {**identity, "name": "hardpoint_c/Box", "parent": 0, "mesh": "m0002"},
        ],
        "meshes": [
            _mesh("m0001", "Gun_mat", {"diffuse": "t0001", "normal": "t0002"}),
            _mesh("m0002", "Box_mat", {}),
        ],
        "textures": [texture, normal],
        "issues": [],
    }
    value.update(overrides)
    return value


def lit_manifest(**overrides: Any) -> dict[str, Any]:
    """Returns ``manifest()`` as format 1.2 with an Interior below the root.

    The Interior is a group ``Interior``, a container ``room`` at (0, 0, -6) with a ``stool``
    object (mesh m0002) and a point, a spot and a directional light. The lights shine along their
    local -Z axis; the root is turned a quarter turn about Y.
    """
    base = {
        "kind": "container",
        "translation": [0.0, 0.0, 0.0],
        "rotation": [0.0, 0.0, 0.0, 1.0],
        "scale": [1.0, 1.0, 1.0],
    }

    def light(name: str, value: dict[str, Any], **extra: Any) -> dict[str, Any]:
        return {**base, "kind": "light", "name": name, "path": f"Interior/room/{name}", "parent": 5, "light": value, **extra}

    point = {"kind": "point", "colour": [1.0, 0.5, 0.25], "intensity": 3.0, "range": 6.0}
    spot = {
        "kind": "spot",
        "colour": [1.0, 1.0, 1.0],
        "intensity": 2.0,
        "inner_cone": 0.0,
        "outer_cone": 0.5235988,
    }
    sun = {"kind": "directional", "colour": [0.9, 0.9, 1.0], "intensity": 1.5}
    data = manifest(version="1.2", **overrides)
    data["nodes"] += [
        {**base, "name": "Interior", "path": "Interior", "parent": 0},
        {**base, "name": "room", "path": "Interior/room", "parent": 4, "translation": [0.0, 0.0, -6.0]},
        {**base, "kind": "object", "name": "stool", "path": "Interior/room/stool", "parent": 5, "mesh": "m0002"},
        light("lamp", point, translation=[0.0, 2.5, 0.0], scale=[2.0, 3.0, 4.0]),
        light("beam", spot),
        light("sun", sun),
    ]
    return data


def interior_manifest(**overrides: Any) -> dict[str, Any]:
    """Returns a format 1.3 manifest of an Interior on its own, without a node of the kind entity.

    The container ``ship_interior`` is the root; below it are the container ``room`` at (0, 0, -6),
    a ``stool`` object (mesh m0001) and a point light ``lamp``.
    """
    base = {
        "kind": "container",
        "translation": [0.0, 0.0, 0.0],
        "rotation": [0.0, 0.0, 0.0, 1.0],
        "scale": [1.0, 1.0, 1.0],
    }
    data = manifest(version="1.3", **overrides)
    data["source"] = {"entity": {"class_name": "ship_interior", "guid": ""}}
    data["nodes"] = [
        {**base, "name": "ship_interior", "path": "ship_interior"},
        {
            **base,
            "name": "room",
            "path": "ship_interior/room",
            "parent": 0,
            "translation": [0.0, 0.0, -6.0],
        },
        {
            **base,
            "kind": "object",
            "name": "stool",
            "path": "ship_interior/room/stool",
            "parent": 1,
            "mesh": "m0001",
        },
        {
            **base,
            "kind": "light",
            "name": "lamp",
            "path": "ship_interior/room/lamp",
            "parent": 1,
            "light": {"kind": "point", "colour": [1.0, 1.0, 1.0], "intensity": 2.0},
        },
    ]
    data["meshes"] = [_mesh("m0001", "Box_mat", {})]
    data["textures"] = []
    return data


def write_package(folder: Path, data: dict[str, Any] | None = None) -> Path:
    """Writes ``data`` (default: ``manifest()``) and the files it names; returns ``scene.json``."""
    data = data if data is not None else manifest()
    (folder / "meshes").mkdir(parents=True, exist_ok=True)
    (folder / "textures").mkdir(parents=True, exist_ok=True)
    for mesh in data["meshes"]:
        (folder / mesh["file"]).write_bytes(
            glb(
                mesh["id"],
                [m["name"] for m in mesh["materials"]],
                extras=any("material" in m for m in mesh["materials"]),
            )
        )
    for texture in data["textures"]:
        (folder / texture["file"]).write_bytes(png())
    path = folder / "scene.json"
    path.write_text(json.dumps(data), encoding="utf-8")
    return path


def glb(
    root_name: str,
    materials: list[str],
    external: dict[str, Any] | None = None,
    extras: bool = False,
) -> bytes:
    """Returns a GLB with a root node, a child with one triangle per material.

    ``external`` entries replace top-level keys of the glTF JSON, e.g. a buffer with a ``uri``.
    ``extras`` gives each material the ``p4kx_index`` extras of format 1.1.
    """
    document = {
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"name": root_name, "children": [1]}, {"name": f"{root_name}_geo", "mesh": 0}],
        "meshes": [
            {
                "name": f"{root_name}_mesh",
                "primitives": [
                    {"attributes": {"POSITION": 0}, "material": slot}
                    for slot in range(len(materials))
                ],
            }
        ],
        "materials": [
            {
                "name": name,
                "pbrMetallicRoughness": {"baseColorFactor": [0.5, 0.5, 0.5, 1.0]},
                **({"extras": {"p4kx_index": slot}} if extras else {}),
            }
            for slot, name in enumerate(materials)
        ],
        "accessors": [
            {
                "bufferView": 0,
                "componentType": 5126,
                "count": 3,
                "type": "VEC3",
                "min": [0, 0, 0],
                "max": [1, 1, 0],
            }
        ],
        "bufferViews": [{"buffer": 0, "byteLength": len(TRIANGLE), "target": 34962}],
        "buffers": [{"byteLength": len(TRIANGLE)}],
    }
    document.update(external or {})
    json_chunk = json.dumps(document).encode("utf-8")
    json_chunk += b" " * (-len(json_chunk) % 4)
    total = 12 + 8 + len(json_chunk) + 8 + len(TRIANGLE)
    return (
        struct.pack("<4sII", b"glTF", 2, total)
        + struct.pack("<I4s", len(json_chunk), b"JSON")
        + json_chunk
        + struct.pack("<I4s", len(TRIANGLE), b"BIN\x00")
        + TRIANGLE
    )


def png() -> bytes:
    """Returns a 1x1 RGBA PNG."""

    def chunk(kind: bytes, body: bytes) -> bytes:
        return (
            struct.pack(">I", len(body))
            + kind
            + body
            + struct.pack(">I", zlib.crc32(kind + body))
        )

    header = struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0)
    pixels = zlib.compress(b"\x00\xff\x80\x40\xff")
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", pixels) + chunk(b"IEND", b"")


def _mesh(mesh_id: str, material: str, textures: dict[str, str]) -> dict[str, Any]:
    return {
        "id": mesh_id,
        "file": f"meshes/{mesh_id}.glb",
        "instances": 1,
        "bytes": 0,
        "vertices": 3,
        "triangles": 1,
        "materials": [{"index": 0, "name": material, "textures": textures}],
    }
