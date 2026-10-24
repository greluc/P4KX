# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""The node groups that many materials share, described as data and created once per file."""

from __future__ import annotations

from dataclasses import dataclass

from .graph_spec import GraphBuilder, GraphSpec, Ref

LAYER_BLEND = "P4KX Layer Blend v1"
WEAR = "P4KX Wear v1"

GROUP_INPUT = "group_input"
GROUP_OUTPUT = "group_output"


@dataclass(frozen=True, slots=True)
class GroupSpec:
    """A node group: its interface as ``(name, socket type, default)`` and its inside."""

    name: str
    inputs: tuple[tuple[str, str, object], ...]
    outputs: tuple[tuple[str, str], ...]
    graph: GraphSpec


def layer_blend() -> GroupSpec:
    """The blend of two surfaces by a factor: colour, roughness and normal."""
    builder = GraphBuilder("group")
    builder.add("NodeGroupInput", key=GROUP_INPUT)
    builder.add("NodeGroupOutput", key=GROUP_OUTPUT)
    colour = builder.mix(Ref(GROUP_INPUT, "Colour A"), Ref(GROUP_INPUT, "Colour B"), Ref(GROUP_INPUT, "Factor"))
    rough = builder.mix(
        Ref(GROUP_INPUT, "Roughness A"),
        Ref(GROUP_INPUT, "Roughness B"),
        Ref(GROUP_INPUT, "Factor"),
        data_type="FLOAT",
    )
    metallic = builder.mix(
        Ref(GROUP_INPUT, "Metallic A"),
        Ref(GROUP_INPUT, "Metallic B"),
        Ref(GROUP_INPUT, "Factor"),
        data_type="FLOAT",
    )
    normal = builder.mix(
        Ref(GROUP_INPUT, "Normal A"),
        Ref(GROUP_INPUT, "Normal B"),
        Ref(GROUP_INPUT, "Factor"),
        data_type="VECTOR",
    )
    unit = builder.add("ShaderNodeVectorMath", props={"operation": "NORMALIZE"})
    builder.link(normal, unit, 0)
    builder.link(colour, GROUP_OUTPUT, "Colour")
    builder.link(rough, GROUP_OUTPUT, "Roughness")
    builder.link(metallic, GROUP_OUTPUT, "Metallic")
    builder.link(Ref(unit, "Vector"), GROUP_OUTPUT, "Normal")
    return GroupSpec(
        LAYER_BLEND,
        (
            ("Colour A", "NodeSocketColor", (0.5, 0.5, 0.5, 1.0)),
            ("Colour B", "NodeSocketColor", (0.5, 0.5, 0.5, 1.0)),
            ("Roughness A", "NodeSocketFloat", 0.5),
            ("Roughness B", "NodeSocketFloat", 0.5),
            ("Metallic A", "NodeSocketFloat", 0.0),
            ("Metallic B", "NodeSocketFloat", 0.0),
            ("Normal A", "NodeSocketVector", (0.0, 0.0, 1.0)),
            ("Normal B", "NodeSocketVector", (0.0, 0.0, 1.0)),
            ("Factor", "NodeSocketFloat", 0.0),
        ),
        (
            ("Colour", "NodeSocketColor"),
            ("Roughness", "NodeSocketFloat"),
            ("Metallic", "NodeSocketFloat"),
            ("Normal", "NodeSocketVector"),
        ),
        builder.spec,
    )


def wear() -> GroupSpec:
    """Wear: a colour tinted by the wear tint where the wear factor is high."""
    builder = GraphBuilder("group")
    builder.add("NodeGroupInput", key=GROUP_INPUT)
    builder.add("NodeGroupOutput", key=GROUP_OUTPUT)
    worn = builder.mix(Ref(GROUP_INPUT, "Colour"), Ref(GROUP_INPUT, "Wear Tint"), 1.0, blend="MULTIPLY")
    result = builder.mix(Ref(GROUP_INPUT, "Colour"), worn, Ref(GROUP_INPUT, "Wear"))
    builder.link(result, GROUP_OUTPUT, "Colour")
    return GroupSpec(
        WEAR,
        (
            ("Colour", "NodeSocketColor", (0.5, 0.5, 0.5, 1.0)),
            ("Wear Tint", "NodeSocketColor", (1.0, 1.0, 1.0, 1.0)),
            ("Wear", "NodeSocketFloat", 0.0),
        ),
        (("Colour", "NodeSocketColor"),),
        builder.spec,
    )


def group_spec(name: str) -> GroupSpec:
    """Returns the description of the shared group ``name``."""
    if name == LAYER_BLEND:
        return layer_blend()
    if name == WEAR:
        return wear()
    raise KeyError(name)
