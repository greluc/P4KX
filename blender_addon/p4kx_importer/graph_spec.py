# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""A node graph as plain data: what a material becomes, before any Blender node exists.

The mapping from a material description to nodes (``material_graph``) produces a ``GraphSpec``;
``materials`` turns it into Blender nodes. Everything here is testable without Blender.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, NamedTuple

Socket = str | int


class Ref(NamedTuple):
    """An output socket of a node of the graph, by node key and socket name or index."""

    key: str
    socket: Socket


@dataclass(slots=True)
class NodeSpec:
    """One node: the Blender node type, its properties, input defaults and what it refers to."""

    key: str
    kind: str
    props: dict[str, Any] = field(default_factory=dict)
    inputs: dict[Socket, Any] = field(default_factory=dict)
    texture: str | None = None
    group: str | None = None
    label: str = ""


@dataclass(frozen=True, slots=True)
class LinkSpec:
    """A link from an output socket of one node to an input socket of another."""

    source: Ref
    target: Ref


@dataclass(slots=True)
class GraphSpec:
    """The nodes and links of one material or node group, and the settings of the material."""

    family: str = ""
    nodes: list[NodeSpec] = field(default_factory=list)
    links: list[LinkSpec] = field(default_factory=list)
    alpha_mode: str = "opaque"
    two_sided: bool = True
    properties: dict[str, str] = field(default_factory=dict)
    warnings: list[str] = field(default_factory=list)

    def node(self, key: str) -> NodeSpec:
        """Returns the node called ``key``."""
        return next(n for n in self.nodes if n.key == key)

    def kinds(self) -> set[str]:
        """Returns the Blender node types the graph uses."""
        return {n.kind for n in self.nodes}

    def groups(self) -> set[str]:
        """Returns the names of the node groups the graph instantiates."""
        return {n.group for n in self.nodes if n.group}

    def linked(self, target: str, socket: Socket) -> Ref | None:
        """Returns the output linked to ``target``'s input ``socket``, if any."""
        return next(
            (l.source for l in self.links if l.target == Ref(target, socket)),
            None,
        )


class GraphBuilder:
    """Adds nodes and links to a ``GraphSpec`` with keys made from the node type."""

    def __init__(self, family: str) -> None:
        self.spec = GraphSpec(family=family)
        self._counts: dict[str, int] = {}

    def add(
        self,
        kind: str,
        *,
        key: str | None = None,
        props: dict[str, Any] | None = None,
        inputs: dict[Socket, Any] | None = None,
        texture: str | None = None,
        group: str | None = None,
        label: str = "",
    ) -> str:
        """Adds a node and returns its key, ``key`` or one made from ``kind``."""
        if key is None:
            count = self._counts.get(kind, 0) + 1
            self._counts[kind] = count
            key = f"{kind}.{count}"
        self.spec.nodes.append(
            NodeSpec(key, kind, props or {}, dict(inputs or {}), texture, group, label)
        )
        return key

    def link(self, source: Ref, target: str, socket: Socket) -> None:
        """Links ``source`` to input ``socket`` of node ``target``."""
        self.spec.links.append(LinkSpec(source, Ref(target, socket)))

    def put(self, target: str, socket: Socket, value: Ref | Any) -> None:
        """Links ``value`` to the input when it is a ``Ref``, else sets it as the default."""
        if isinstance(value, Ref):
            self.link(value, target, socket)
        else:
            self.spec.node(target).inputs[socket] = value

    def mix(
        self,
        a: Ref | Any,
        b: Ref | Any,
        factor: Ref | float,
        *,
        blend: str = "MIX",
        data_type: str = "RGBA",
    ) -> Ref:
        """Adds a Mix node and returns its result."""
        key = self.add(
            "ShaderNodeMix",
            props={"data_type": data_type, "blend_type": blend, "clamp_result": True},
        )
        self.put(key, "A", a)
        self.put(key, "B", b)
        self.put(key, "Factor", factor)
        return Ref(key, "Result")

    def math(self, operation: str, a: Ref | float, b: Ref | float | None = None) -> Ref:
        """Adds a Math node and returns its value."""
        key = self.add("ShaderNodeMath", props={"operation": operation})
        self.put(key, 0, a)
        if b is not None:
            self.put(key, 1, b)
        return Ref(key, 0)
