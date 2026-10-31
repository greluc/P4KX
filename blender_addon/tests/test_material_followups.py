# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Tests of layer shares without a blend source, missing textures and decal stencils."""

from __future__ import annotations

from collections.abc import Callable

import pytest

from p4kx_importer.graph_groups import LAYER_BLEND
from p4kx_importer.graph_spec import GraphSpec
from p4kx_importer.material_graph import BuildContext, build_graph
from p4kx_importer.material_model import parse_materials, parse_palettes
from synthetic_materials import description, entry, layer, slot

WEAR_BASE = "WearBlendBase"


def single(
    descriptions: list[dict],
    palette: dict | None = None,
    loadable: Callable[[str], bool] | None = None,
) -> GraphSpec:
    materials = parse_materials(descriptions, [])
    palettes = parse_palettes([palette], []) if palette else {}
    context = BuildContext(materials, palettes.get("p0001"), {}, loadable)
    return build_graph(materials[descriptions[0]["id"]], context)


def layered(
    wear: str | None,
    count: int,
    shares: list[float] | None = None,
    reads_vertex_colours: bool | None = False,
    blend_texture: str | None = None,
    loadable: Callable[[str], bool] | None = None,
) -> GraphSpec:
    """A HardSurface of ``count`` layers, as the package writer would state it.

    ``blend_texture`` gives it a blend map; ``reads_vertex_colours`` is the writer's statement.
    """
    layers = [
        layer(f"Layer{at}", f"x{at + 2:04d}", tint_colour=[0.1 * (at + 1)] * 3)
        for at in range(count)
    ]
    fields: dict = {"layers": layers}
    if wear is not None:
        fields["parameters"] = [{"name": WEAR_BASE, "value": wear}]
    if reads_vertex_colours is not None:
        fields["reads_vertex_colours"] = reads_vertex_colours
    if shares is not None:
        fields["layer_shares"] = shares
    if blend_texture is not None:
        fields["textures"] = [slot("TexSlot12", "blend", blend_texture)]
    raw = [
        description("x0001", "HardSurface", **fields),
        *(description(f"x{at + 2:04d}", "Layer") for at in range(count)),
    ]
    return single(raw, loadable=loadable)


def blends(graph: GraphSpec) -> list:
    return [node for node in graph.nodes if node.group == LAYER_BLEND]


def no_texture(_texture: str) -> bool:
    return False


def test_a_failed_blend_map_blends_the_layers_by_the_stated_shares() -> None:
    graph = layered("0.25", 2, [0.75, 0.25], blend_texture="t0001", loadable=no_texture)
    [blend] = blends(graph)
    assert blend.inputs["Factor"] == pytest.approx(0.25)
    assert blend.inputs["Colour A"] == pytest.approx((0.1, 0.1, 0.1))
    assert blend.inputs["Colour B"] == pytest.approx((0.2, 0.2, 0.2))


def test_stated_shares_are_divided_between_three_layers() -> None:
    graph = layered("0.6", 3, [0.4, 0.3, 0.3], blend_texture="t0001", loadable=no_texture)
    first, second = blends(graph)
    assert first.inputs["Factor"] == pytest.approx(0.3 / 0.7)
    assert second.inputs["Factor"] == pytest.approx(0.3)


def test_a_layer_without_a_stated_share_is_left_out() -> None:
    graph = layered("0", 2, [1.0, 0.0], blend_texture="t0001", loadable=no_texture)
    assert blends(graph) == []


def test_an_older_package_gets_the_same_shares_from_the_wear_parameter() -> None:
    graph = layered("0.3", 2, None, blend_texture="t0001", loadable=no_texture)
    [blend] = blends(graph)
    assert blend.inputs["Factor"] == pytest.approx(0.3)
    assert blends(layered("0", 2, None, blend_texture="t0001", loadable=no_texture)) == []


def test_a_single_layer_without_wear_weight_shows_that_layer() -> None:
    for shares in ([1.0], None):
        graph = layered("1", 1, shares)
        assert graph.node("bsdf").inputs["Base Color"] == pytest.approx((0.1, 0.1, 0.1))


def test_a_missing_first_layer_keeps_the_own_colours() -> None:
    raw = [
        description(
            "x0001",
            "HardSurface",
            diffuse=[0.7, 0.6, 0.5],
            layers=[layer("Layer0", "x9999")],
            reads_vertex_colours=False,
            layer_shares=[0.0],
        )
    ]
    graph = single(raw)
    assert graph.node("bsdf").inputs["Base Color"] == pytest.approx((0.7, 0.6, 0.5))


def test_stated_shares_that_do_not_match_the_layers_are_ignored() -> None:
    graph = layered("0.3", 2, [1.0], blend_texture="t0001", loadable=no_texture)
    [blend] = blends(graph)
    assert blend.inputs["Factor"] == pytest.approx(0.3)


def test_two_layers_without_a_blend_map_blend_by_the_vertex_colours() -> None:
    graph = layered("0.25", 2, [0.75, 0.25], reads_vertex_colours=True)
    [blend] = blends(graph)
    assert any(node.kind == "ShaderNodeVertexColor" for node in graph.nodes)
    factor = graph.linked(blend.key, "Factor")
    assert factor is not None and graph.node(factor.key).kind == "ShaderNodeMath"


def test_a_texture_that_does_not_load_falls_back_to_the_factors() -> None:
    raw = [
        description(
            "x0001",
            "Illum",
            diffuse=[0.2, 0.4, 0.6],
            textures=[slot("TexSlot1", "diffuse", "t0001"), slot("TexSlot2", "normal", "t0002")],
        )
    ]
    graph = single(raw, loadable=lambda texture: texture == "t0001")
    assert [n.texture for n in graph.nodes if n.kind == "ShaderNodeTexImage"] == ["t0001"]
    assert graph.linked("bsdf", "Normal") is None
    gone = single(raw, loadable=no_texture)
    assert not [n for n in gone.nodes if n.kind == "ShaderNodeTexImage"]
    assert gone.node("bsdf").inputs["Base Color"] == pytest.approx((0.2, 0.4, 0.6))


def test_only_the_textures_a_material_names_are_asked_for() -> None:
    asked: list[str] = []

    def loadable(texture: str) -> bool:
        asked.append(texture)
        return True

    raw = [description("x0001", "Illum", textures=[slot("TexSlot2", "normal", "t0002")])]
    single(raw, loadable=loadable)
    assert set(asked) == {"t0002"}


def test_every_texture_counts_as_loadable_without_a_predicate() -> None:
    raw = [description("x0001", "Illum", textures=[slot("TexSlot2", "normal", "t0002")])]
    assert single(raw).linked("bsdf", "Normal") is not None


def test_the_textures_of_a_layer_file_that_do_not_load_fall_back_too() -> None:
    raw = [
        description(
            "x0001", "HardSurface", layers=[layer("Layer0", "x0002", tint_colour=[0.3] * 3)]
        ),
        description("x0002", "Layer", textures=[slot("TexSlot1", "diffuse", "t0005")]),
    ]
    graph = single(raw, loadable=no_texture)
    assert not [n for n in graph.nodes if n.kind == "ShaderNodeTexImage"]
    assert graph.node("bsdf").inputs["Base Color"] == pytest.approx((0.3, 0.3, 0.3))


def decal(gen_mask: list[str] | None = None, **parameters: str) -> dict:
    return description(
        "x0001",
        "MeshDecal",
        gen_mask=["STENCIL_MAP"] if gen_mask is None else gen_mask,
        textures=[slot("TexSlot1", "diffuse", None, file="$TintPaletteDecal")],
        parameters=[{"name": name, "value": value} for name, value in parameters.items()],
    )


def test_a_stencil_decal_without_a_palette_takes_its_stencil_colour() -> None:
    graph = single([decal(StencilDiffuseColor="0.1,0.2,0.3")])
    assert graph.node("bsdf").inputs["Base Color"] == pytest.approx((0.1, 0.2, 0.3))


def test_a_decal_without_a_stencil_map_ignores_the_stencil_parameters() -> None:
    graph = single([decal([], StencilDiffuseColor="0.1,0.2,0.3", StencilOpacity="0.5")])
    assert graph.node("bsdf").inputs["Base Color"] == pytest.approx((1.0, 1.0, 1.0))
    assert graph.linked("bsdf", "Alpha") is None
    assert "Alpha" not in graph.node("bsdf").inputs


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
