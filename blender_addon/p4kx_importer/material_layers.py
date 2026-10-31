# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""The layered surface of a material: its layers blended by the blend mask, wear or vertex colours."""

from __future__ import annotations

from .graph_groups import LAYER_BLEND, WEAR
from .graph_spec import Ref
from .material_model import Colour, Layer, MaterialDesc
from .material_surface import (
    MASKED_LAYERS,
    MAX_BLENDED_LAYERS,
    BuildContext,
    Surface,
    reads_vertex_colours,
)

Resolved = tuple[int, Layer, MaterialDesc]
# A weight sum at or below this is no weight at all (``f32::EPSILON`` of the Rust writer).
WEIGHT_EPSILON = 1.1920929e-07


class LayerBlending:
    """Builds the blend of the layers of ``desc``; mixed into the material maker (ADR-0100, ADR-0207)."""

    def _layered(self) -> Surface:
        desc = self.desc
        context: BuildContext = self.context
        resolved: list[Resolved] = [
            (at, layer, context.materials[layer.material])
            for at, layer in enumerate(desc.layers)
            if layer.material in context.materials
        ]
        resolved = [(at, layer, context.usable(layer_desc)) for at, layer, layer_desc in resolved]
        if not resolved:
            return self._base()
        if desc.slot("blend"):
            return self._by_precedence(resolved)
        if not (desc.slot("mask") or reads_vertex_colours(desc)):
            return self._by_position(resolved)
        pairs = [(layer, layer_desc) for _, layer, layer_desc in resolved]
        if desc.slot("diffuse"):
            foundation = self._base()
        else:
            foundation = self._layer_surface(*pairs[0])
            pairs = pairs[1:]
        if len(pairs) > MAX_BLENDED_LAYERS:
            self.builder.spec.warnings.append(
                f"Material {desc.name or desc.id!r} has {len(pairs) + 1} layers: "
                f"only the first {MAX_BLENDED_LAYERS + 1} are blended."
            )
            pairs = pairs[:MAX_BLENDED_LAYERS]
        if not pairs:
            return foundation
        channels = self._blend_channels()
        wear = self._wear_for([layer for layer, _ in pairs])
        current = foundation
        for channel, (layer, layer_desc) in zip(channels, pairs):
            current = self._blend(current, self._worn_surface(layer, layer_desc, wear), channel)
        return current

    def _by_precedence(self, resolved: list[Resolved]) -> Surface:
        """Blends the base layers of a blend mask by precedence, then the layers after them.

        A base layer is one of the first ``MASKED_LAYERS`` positions of ``MatLayers``. Red takes the
        first, green the second and blue the third, each above the ones after it; a missing layer is
        skipped and the last base layer present takes what no channel claims. The layers after the
        base layers take their share by position, as the glTF bake does.
        """
        desc = self.desc
        base = [item for item in resolved if item[0] < MASKED_LAYERS]
        if not base:
            return self._base()
        wear = self._wear_for([layer for _, layer, _ in resolved])
        surfaces = [self._worn_surface(layer, layer_desc, wear) for _, layer, layer_desc in base]
        current = surfaces[-1]
        if len(surfaces) > 1:
            channels = self._blend_channels()
            for index in range(len(surfaces) - 2, -1, -1):
                current = self._blend(current, surfaces[index], channels[base[index][0]])
        current = current._replace(level=surfaces[0].level)
        shares = self._rest_shares([at for at, _, _ in resolved])
        rest = [item for item in resolved if item[0] >= MASKED_LAYERS]
        mass = 1.0 - sum(shares)
        for (_, layer, layer_desc), share in zip(rest, shares):
            mass += share
            if share > 0.0:
                above = self._worn_surface(layer, layer_desc, wear)
                current = self._blend(current, above, min(share / mass, 1.0))
        if desc.slot("diffuse"):
            current = current._replace(colour=self._base().colour)
        return current

    def _by_position(self, resolved: list[Resolved]) -> Surface:
        """Blends the layers by their position weights, for a material without a blend source.

        The first layer weighs ``1 - WearBlendBase`` and the others ``WearBlendBase`` between them
        (ADR-0097), as the package states them. With no weight at all the first layer takes
        everything; the material keeps its own colours only when that layer is missing.
        """
        weights = self._position_weights([at for at, _, _ in resolved])
        if weights is None:
            return self._base()
        wear = self._wear_for([layer for _, layer, _ in resolved])
        surfaces = [self._worn_surface(layer, layer_desc, wear) for _, layer, layer_desc in resolved]
        current, mass = surfaces[0], weights[0]
        for surface, weight in zip(surfaces[1:], weights[1:]):
            if weight > 0.0:
                mass += weight
                current = self._blend(current, surface, min(weight / mass, 1.0))
        if self.desc.slot("diffuse"):
            current = current._replace(colour=self._base().colour)
        return current

    def _position_weights(self, present: list[int]) -> list[float] | None:
        """Returns the weight of each layer in ``present``, or ``None`` for the own colours.

        A package of format 1.5 states the shares (``Material::layer_shares``). For an older one
        the same rule is applied here: no weight at all gives the first layer everything when it
        is present, else the own colours.
        """
        desc = self.desc
        if desc.layer_shares is not None:
            weights = [desc.layer_shares[at] for at in present]
            return weights if sum(weights) > WEIGHT_EPSILON else None
        wear = min(max(desc.number("WearBlendBase") or 0.0, 0.0), 1.0)
        others = max(len(desc.layers) - 1, 1)
        weights = [1.0 - wear if at == 0 else wear / others for at in present]
        total = sum(weights)
        if total > WEIGHT_EPSILON:
            return [weight / total for weight in weights]
        if present and present[0] == 0:
            return [1.0, *(0.0 for _ in present[1:])]
        return None

    def _rest_shares(self, present: list[int]) -> list[float]:
        """Returns the share by position of each layer after the base layers, as the bake has it."""
        weights = self._position_weights(present)
        if weights is None:
            return [0.0 for at in present if at >= MASKED_LAYERS]
        return [weight for at, weight in zip(present, weights) if at >= MASKED_LAYERS]

    def _wear_for(self, layers: list[Layer]) -> Ref | float | None:
        if any(layer.wear_tint for layer in layers):
            return self._wear_factor()
        return None

    def _worn_surface(
        self, layer: Layer, layer_desc: MaterialDesc, wear: Ref | float | None
    ) -> Surface:
        surface = self._layer_surface(layer, layer_desc)
        if layer.wear_tint and wear is not None:
            surface = surface._replace(colour=self._worn(surface.colour, layer.wear_tint, wear))
        return surface

    def _blend_channels(self) -> list[Ref]:
        b = self.builder
        if slot := self.desc.slot("blend") or self.desc.slot("mask"):
            source = self._image(slot, 1.0)
            split = b.add("ShaderNodeSeparateColor")
            b.link(Ref(source, "Color"), split, "Color")
            return [Ref(split, "Red"), Ref(split, "Green"), Ref(split, "Blue"), Ref(source, "Alpha")]
        source = b.add("ShaderNodeVertexColor")
        split = b.add("ShaderNodeSeparateColor")
        b.link(Ref(source, "Color"), split, "Color")
        worn = b.math("SUBTRACT", 1.0, Ref(split, "Red"), clamp=True)
        if (base := self.desc.number("WearBlendBase")) is not None:
            worn = b.math("MULTIPLY", worn, base, clamp=True)
        return [worn, Ref(split, "Green"), Ref(split, "Blue"), Ref(source, "Alpha")]

    def _wear_factor(self) -> Ref | float | None:
        b = self.builder
        base = self.desc.number("WearBlendBase")
        if slot := self.desc.slot("wear"):
            bw = b.add("ShaderNodeRGBToBW")
            b.link(Ref(self._image(slot, 1.0), "Color"), bw, "Color")
            value = Ref(bw, "Val")
            return value if base is None else b.math("MULTIPLY", value, base)
        return base

    def _worn(self, colour: Ref | Colour, tint: Colour, factor: Ref | float) -> Ref:
        b = self.builder
        node = b.add("ShaderNodeGroup", group=WEAR)
        b.put(node, "Colour", colour)
        b.put(node, "Wear Tint", tint)
        b.put(node, "Wear", factor)
        return Ref(node, "Colour")

    def _blend(self, below: Surface, above: Surface, factor: Ref | float) -> Surface:
        b = self.builder
        node = b.add("ShaderNodeGroup", group=LAYER_BLEND)
        b.put(node, "Colour A", below.colour)
        b.put(node, "Colour B", above.colour)
        b.put(node, "Roughness A", below.roughness)
        b.put(node, "Roughness B", above.roughness)
        b.put(node, "Metallic A", below.metallic)
        b.put(node, "Metallic B", above.metallic)
        b.put(node, "Factor", factor)
        colour, roughness, metallic = Ref(node, "Colour"), Ref(node, "Roughness"), Ref(node, "Metallic")
        normal_a, normal_b = below.normal, above.normal
        if normal_a is None and normal_b is None:
            return Surface(colour, roughness, None, metallic, below.level)
        geometry = b.add("ShaderNodeNewGeometry")
        b.put(node, "Normal A", normal_a or Ref(geometry, "Normal"))
        b.put(node, "Normal B", normal_b or Ref(geometry, "Normal"))
        return Surface(colour, roughness, Ref(node, "Normal"), metallic, below.level)
