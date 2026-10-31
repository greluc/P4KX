# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Tests of layer weights without a blend source, missing textures and decal stencils."""

from __future__ import annotations

import pytest

from p4kx_importer.graph_groups import LAYER_BLEND
from p4kx_importer.graph_spec import GraphSpec
from p4kx_importer.material_graph import BuildContext, build_graph
from p4kx_importer.material_model import parse_materials, parse_palettes
from synthetic_materials import description, entry, layer, slot

WEAR_BASE = "WearBlendBase"


def single(
    descriptions: list[dict], palette: dict | None = None, loadable: frozenset[str] | None = None
) -> GraphSpec:
    materials = parse_materials(descriptions, [])
    palettes = parse_palettes([palette], []) if palette else {}
    context = BuildContext(materials, palettes.get("p0001"), {}, loadable)
    return build_graph(materials[descriptions[0]["id"]], context)


def stacked(wear: str | None, count: int = 2, **fields: object) -> GraphSpec:
    """A HardSurface of ``count`` layers that has no blend map, no mask and no vertex colours."""
    layers = [layer(f"Layer{at}", f"x{at + 2:04d}", tint_colour=[0.1 * (at + 1)] * 3) for at in range(count)]
    parameters = [] if wear is None else [{"name": WEAR_BASE, "value": wear}]
    raw = [
        description("x0001", "HardSurface", layers=layers, parameters=parameters, reads_vertex_colours=False, **fields),
        *(description(f"x{at + 2:04d}", "Layer") for at in range(count)),
    ]
    return single(raw)


def blends(graph: GraphSpec) -> list:
    return [node for node in graph.nodes if node.group == LAYER_BLEND]


def test_without_a_blend_source_the_second_layer_takes_the_wear_share() -> None:
    graph = stacked("0.3")
    [blend] = blends(graph)
    assert blend.inputs["Factor"] == pytest.approx(0.3)
    assert blend.inputs["Colour A"] == pytest.approx((0.1, 0.1, 0.1))
    assert blend.inputs["Colour B"] == pytest.approx((0.2, 0.2, 0.2))


def test_without_a_blend_source_the_wear_share_is_divided_between_the_other_layers() -> None:
    graph = stacked("0.6", count=3)
    first, second = blends(graph)
    assert first.inputs["Factor"] == pytest.approx(0.3 / 0.7)
    assert second.inputs["Factor"] == pytest.approx(0.3)


def test_without_wear_the_first_layer_is_the_whole_surface() -> None:
    assert blends(stacked(None)) == []
    assert blends(stacked("0")) == []


def test_without_a_first_layer_weight_the_others_take_the_surface() -> None:
    graph = stacked("1")
    [blend] = blends(graph)
    assert blend.inputs["Factor"] == pytest.approx(1.0)


def test_no_weight_at_all_keeps_the_material_colours() -> None:
    raw = [
        description(
            "x0001",
            "HardSurface",
            diffuse=[0.7, 0.6, 0.5],
            layers=[layer("Layer0", "x9999")],
            reads_vertex_colours=False,
        )
    ]
    graph = single(raw)
    assert graph.node("bsdf").inputs["Base Color"] == pytest.approx((0.7, 0.6, 0.5))


def test_vertex_colours_still_blend_by_their_channels() -> None:
    layers = [layer("Layer0", "x0002"), layer("Layer1", "x0003")]
    raw = [
        description("x0001", "HardSurface", layers=layers, reads_vertex_colours=True),
        description("x0002", "Layer"),
        description("x0003", "Layer"),
    ]
    graph = single(raw)
    assert any(node.kind == "ShaderNodeVertexColor" for node in graph.nodes)


def test_a_texture_that_does_not_load_falls_back_to_the_factors() -> None:
    raw = [
        description(
            "x0001",
            "Illum",
            diffuse=[0.2, 0.4, 0.6],
            textures=[slot("TexSlot1", "diffuse", "t0001"), slot("TexSlot2", "normal", "t0002")],
        )
    ]
    graph = single(raw, loadable=frozenset({"t0001"}))
    assert [n.texture for n in graph.nodes if n.kind == "ShaderNodeTexImage"] == ["t0001"]
    assert graph.linked("bsdf", "Normal") is None
    gone = single(raw, loadable=frozenset())
    assert not [n for n in gone.nodes if n.kind == "ShaderNodeTexImage"]
    assert gone.node("bsdf").inputs["Base Color"] == pytest.approx((0.2, 0.4, 0.6))


def test_every_texture_counts_as_loadable_without_a_list() -> None:
    raw = [description("x0001", "Illum", textures=[slot("TexSlot2", "normal", "t0002")])]
    assert single(raw).linked("bsdf", "Normal") is not None


def test_the_textures_of_a_layer_file_that_do_not_load_fall_back_too() -> None:
    raw = [
        description("x0001", "HardSurface", layers=[layer("Layer0", "x0002", tint_colour=[0.3] * 3)]),
        description("x0002", "Layer", textures=[slot("TexSlot1", "diffuse", "t0005")]),
    ]
    graph = single(raw, loadable=frozenset())
    assert not [n for n in graph.nodes if n.kind == "ShaderNodeTexImage"]
    assert graph.node("bsdf").inputs["Base Color"] == pytest.approx((0.3, 0.3, 0.3))


def decal(**parameters: str) -> dict:
    return description(
        "x0001",
        "MeshDecal",
        textures=[slot("TexSlot1", "diffuse", None, file="$TintPaletteDecal")],
        parameters=[{"name": name, "value": value} for name, value in parameters.items()],
    )


def test_a_decal_without_a_palette_takes_its_stencil_colour() -> None:
    graph = single([decal(StencilDiffuseColor="0.1,0.2,0.3")])
    assert graph.node("bsdf").inputs["Base Color"] == pytest.approx((0.1, 0.2, 0.3))


def test_a_palette_entry_wins_over_the_stencil_colour() -> None:
    palette = {
        "id": "p0001",
        "name": "p",
        "primary": entry([0.5, 0.5, 0.5], 100.0),
        "secondary": entry([0.5, 0.5, 0.5], 100.0),
        "tertiary": entry([0.5, 0.5, 0.5], 100.0),
        "glass": [1.0, 1.0, 1.0],
    }
    graph = single([decal(StencilDiffuseColor="0.1,0.2,0.3")], palette)
    assert graph.node("bsdf").inputs["Base Color"] == pytest.approx((0.5, 0.5, 0.5))


def test_a_malformed_stencil_colour_is_ignored() -> None:
    graph = single([decal(StencilDiffuseColor="red")])
    assert graph.node("bsdf").inputs["Base Color"] == pytest.approx((1.0, 1.0, 1.0))


def test_the_stencil_opacity_scales_the_decal_alpha() -> None:
    graph = single([decal(StencilOpacity="0.5")])
    assert graph.node("bsdf").inputs["Alpha"] == pytest.approx(0.5)
