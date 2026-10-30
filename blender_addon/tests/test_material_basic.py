# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Tests of the description made for a material that has none, without Blender."""

from __future__ import annotations

from pathlib import Path

from p4kx_importer.material_basic import basic_description
from p4kx_importer.material_graph import BuildContext, build_graph
from p4kx_importer.material_model import BLACK, DIELECTRIC, WHITE
from p4kx_importer.package import MeshMaterial, Package, Texture


def package_with(**alphas: str) -> Package:
    textures = {tid: Texture(tid, f"{tid}.png", "srgb", alpha) for tid, alpha in alphas.items()}
    return Package(Path("."), (1, 0), "Test", [], {}, textures)


def entry(textures: dict[str, str]) -> MeshMaterial:
    return MeshMaterial(index=3, name="skin", textures=textures)


def test_diffuse_specular_and_normal_give_a_neutral_illum_description() -> None:
    loaded = package_with(t0001="none", t0002="none", t0003="none")
    desc = basic_description(
        entry({"diffuse": "t0001", "specular": "t0002", "normal": "t0003"}), loaded
    )
    assert desc.shader == "Illum" and desc.id == "basic3" and desc.name == "skin"
    assert (desc.diffuse, desc.specular, desc.emissive) == (WHITE, DIELECTRIC, BLACK)
    assert desc.alpha_mode == "opaque" and desc.palette is None and not desc.layers
    assert {s.usage: s.texture for s in desc.textures} == {
        "diffuse": "t0001",
        "specular": "t0002",
        "normal": "t0003",
    }


def test_a_diffuse_with_opacity_alpha_blends_and_one_without_is_opaque() -> None:
    loaded = package_with(t0001="opacity", t0002="none")
    assert basic_description(entry({"diffuse": "t0001"}), loaded).alpha_mode == "blend"
    assert basic_description(entry({"diffuse": "t0002"}), loaded).alpha_mode == "opaque"


def test_without_textures_the_defaults_remain() -> None:
    desc = basic_description(entry({}), package_with())
    assert desc.textures == () and desc.alpha_mode == "opaque"


def test_other_roles_and_unknown_textures_are_left_out() -> None:
    loaded = package_with(t0001="none")
    desc = basic_description(entry({"mask": "t0001", "diffuse": "t0009"}), loaded)
    assert desc.textures == ()


def test_the_blend_alpha_mode_gives_the_same_graph_as_a_described_blend_material() -> None:
    loaded = package_with(t0001="opacity")
    desc = basic_description(entry({"diffuse": "t0001"}), loaded)
    context = BuildContext({}, None, {"t0001": "opacity"})
    graph = build_graph(desc, context)
    assert graph.alpha_mode == "blend"
    assert graph.linked("bsdf", "Alpha") is not None
