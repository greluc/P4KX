# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Tests of painted layers, wear, detail maps and the blend mask precedence, without Blender."""

from __future__ import annotations

import pytest

from p4kx_importer.graph_groups import LAYER_BLEND
from p4kx_importer.graph_spec import GraphSpec, Ref
from p4kx_importer.material_graph import BuildContext, build_graph
from p4kx_importer.material_model import parse_materials, parse_palettes
from synthetic_materials import description, entry, layer, slot

DARK = [0.05, 0.05, 0.05]


def single(descriptions: list[dict], palette: dict | None = None) -> GraphSpec:
    materials = parse_materials(descriptions, [])
    palettes = parse_palettes([palette], []) if palette else {}
    return build_graph(
        materials[descriptions[0]["id"]],
        BuildContext(materials, palettes.get("p0001"), {}),
    )


def paint() -> dict:
    return {
        "id": "p0001",
        "name": "dark",
        "primary": entry(DARK, 200.0),
        "secondary": entry(DARK, 200.0),
        "tertiary": entry(DARK, 200.0),
        "glass": [1.0, 1.0, 1.0],
    }


def bright_specular_layer(material_id: str) -> dict:
    return description(material_id, "Layer", textures=[slot("TexSlot1", "specular", "t0001")])


def test_a_tinted_layer_with_a_bright_specular_map_keeps_its_tint() -> None:
    raw = [
        description("x0001", "HardSurface", layers=[layer("Primary", "x0002", tint_colour=DARK)]),
        bright_specular_layer("x0002"),
    ]
    graph = single(raw)
    assert graph.node("bsdf").inputs["Base Color"] == pytest.approx(tuple(DARK))
    assert graph.node("bsdf").inputs["Metallic"] == 0.0
    level = graph.linked("bsdf", "Specular IOR Level")
    assert level is not None
    assert graph.node(level.key).inputs[1] == pytest.approx(12.5 * 0.04)


def test_a_palette_layer_with_a_bright_specular_map_keeps_the_palette_tint() -> None:
    raw = [
        description("x0001", "HardSurface", layers=[layer("Primary", "x0002", palette="primary")]),
        bright_specular_layer("x0002"),
    ]
    graph = single(raw, paint())
    assert graph.node("bsdf").inputs["Base Color"] == pytest.approx(tuple(DARK))
    assert graph.node("bsdf").inputs["Metallic"] == 0.0


def test_a_palette_entry_with_a_bright_specular_colour_paints_a_metal() -> None:
    metal = {"tint": DARK, "specular": [0.6] * 3, "glossiness": 200.0}
    raw = [
        description("x0001", "HardSurface", layers=[layer("Primary", "x0002", palette="primary")]),
        bright_specular_layer("x0002"),
    ]
    graph = single(raw, {**paint(), "primary": metal})
    assert graph.node("bsdf").inputs["Base Color"] == pytest.approx((0.6, 0.6, 0.6))
    assert graph.node("bsdf").inputs["Metallic"] == 1.0


def test_an_untinted_layer_with_a_bright_specular_map_stays_a_metal() -> None:
    raw = [
        description("x0001", "HardSurface", layers=[layer("Primary", "x0002")]),
        bright_specular_layer("x0002"),
    ]
    metallic = single(raw).linked("bsdf", "Metallic")
    assert metallic is not None


def test_only_the_red_channel_of_a_packed_detail_map_overlays_the_colour() -> None:
    raw = [description("x0001", "Illum", textures=[slot("TexSlot7", "detail", "t0001")])]
    graph = single(raw)
    colour = graph.linked("bsdf", "Base Color")
    assert colour is not None
    overlay = graph.node(colour.key)
    assert overlay.props["blend_type"] == "OVERLAY"
    detail = graph.linked(overlay.key, "B")
    assert detail is not None and detail.socket == "Red"
    assert graph.node(detail.key).kind == "ShaderNodeSeparateColor"


def test_vertex_red_is_pristine_paint_and_wear_blend_base_scales_the_wear() -> None:
    raw = [
        description(
            "x0001",
            "HardSurface",
            layers=[layer("Primary", "x0002"), layer("Wear", "x0002")],
            parameters=[{"name": "WearBlendBase", "value": "0.3"}],
        ),
        description("x0002", "Layer", diffuse=[0.3, 0.3, 0.3]),
    ]
    graph = single(raw)
    blend = next(n for n in graph.nodes if n.group == LAYER_BLEND)
    factor = graph.linked(blend.key, "Factor")
    assert factor is not None
    scale = graph.node(factor.key)
    assert scale.props["operation"] == "MULTIPLY" and scale.inputs[1] == pytest.approx(0.3)
    inverted = graph.linked(scale.key, 0)
    assert inverted is not None
    node = graph.node(inverted.key)
    assert node.props["operation"] == "SUBTRACT" and node.inputs[0] == 1.0
    assert graph.linked(node.key, 1) is not None and graph.linked(node.key, 1).socket == "Red"


def test_the_blend_mask_of_layer_blend_v2_takes_red_over_green_over_blue_over_none() -> None:
    names = ["BaseLayer1", "BaseLayer2", "BaseLayer3", "BaseLayer4", "WearLayer1"]
    colours = [[0.9, 0.0, 0.0], [0.0, 0.9, 0.0], [0.0, 0.0, 0.9], [0.1, 0.1, 0.1], [1.0, 1.0, 1.0]]
    layers = [layer(name, f"x{at + 2:04d}", tint_colour=c) for at, (name, c) in enumerate(zip(names, colours))]
    raw = [
        description("x0001", "LayerBlend_V2", textures=[slot("TexSlot12", "blend", "t0001")], layers=layers),
        *(description(f"x{at + 2:04d}", "Layer") for at in range(len(names))),
    ]
    graph = single(raw)
    blends = [n for n in graph.nodes if n.group == LAYER_BLEND]
    assert len(blends) == 3
    top = graph.linked("bsdf", "Base Color")
    assert top is not None
    node = graph.node(top.key)
    assert node.inputs["Colour B"] == pytest.approx((0.9, 0.0, 0.0))
    factor = graph.linked(node.key, "Factor")
    assert factor is not None and factor.socket == "Red"
    below = graph.linked(node.key, "Colour A")
    assert below is not None
    assert graph.linked(below.key, "Factor") == Ref(factor.key, "Green")
    bottom = graph.linked(below.key, "Colour A")
    assert bottom is not None
    assert graph.node(bottom.key).inputs["Colour A"] == pytest.approx((0.1, 0.1, 0.1))
    assert graph.linked(bottom.key, "Factor") == Ref(factor.key, "Blue")
