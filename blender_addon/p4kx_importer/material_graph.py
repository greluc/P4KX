# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Maps a material description to a node graph (ADR-0100), without Blender.

The mapping is a pure function of the description, the layer descriptions it names, the mesh's
palette and the alpha meaning of the textures, so all of it is tested without Blender.
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
from typing import NamedTuple

from .graph_groups import LAYER_BLEND, WEAR
from .graph_spec import GraphBuilder, GraphSpec, Ref
from .material_model import WHITE, Colour, Layer, MaterialDesc, Palette, PaletteColours, TextureSlot

SURFACE_SHADERS = frozenset({"hardsurface", "illum", "layerblend", "layerblend_v2", "layer", "simple"})
GLASS_SHADERS = frozenset({"glasspbr", "glass"})
DECAL_SHADERS = frozenset({"meshdecal"})
HOLOGRAM_SHADERS = frozenset({"hologram", "hologramcig"})
NODRAW_SHADERS = frozenset({"nodraw", "nodraw_proxy", "proxy"})
MAX_BLENDED_LAYERS = 4
TINT_PALETTE_DECAL = "$TintPaletteDecal"
MAX_PROPERTIES = 64


@dataclass(frozen=True, slots=True)
class BuildContext:
    """What a graph needs besides the description: layer files, the mesh's palette, alphas."""

    materials: Mapping[str, MaterialDesc]
    palette: Palette | None
    texture_alpha: Mapping[str, str]


class Surface(NamedTuple):
    """The colour, roughness and normal of a surface; ``normal`` is ``None`` without a map."""

    colour: Ref | Colour
    roughness: Ref | float
    normal: Ref | None


def family_of(shader: str) -> str:
    """Returns ``surface``, ``glass``, ``decal``, ``hologram``, ``nodraw`` or ``unknown``."""
    name = shader.strip().lower()
    if name in SURFACE_SHADERS:
        return "surface"
    if name in GLASS_SHADERS:
        return "glass"
    if name in DECAL_SHADERS:
        return "decal"
    if name in HOLOGRAM_SHADERS:
        return "hologram"
    if name in NODRAW_SHADERS:
        return "nodraw"
    return "unknown"


def build_graph(desc: MaterialDesc, context: BuildContext) -> GraphSpec:
    """Returns the node graph of ``desc`` as the game would draw it with ``context.palette``."""
    return _Maker(desc, context).build()


class _Maker:
    def __init__(self, desc: MaterialDesc, context: BuildContext) -> None:
        self.desc = desc
        self.context = context
        self.family = family_of(desc.shader)
        self.builder = GraphBuilder(self.family)
        self._images: dict[tuple[str, float, tuple[float, float], tuple[float, float]], str] = {}

    def build(self) -> GraphSpec:
        desc, spec = self.desc, self.builder.spec
        spec.two_sided = desc.two_sided
        spec.properties = self._properties()
        if desc.no_draw or self.family == "nodraw":
            return self._nodraw()
        if self.family == "unknown":
            spec.warnings.append(
                f"Shader {desc.shader!r} of material {desc.name or desc.id!r} is not known: "
                "a basic material is built."
            )
        surface = self._layered() if self.family == "surface" else self._base()
        self._principled(surface)
        return spec

    def _properties(self) -> dict[str, str]:
        properties = {
            "p4kx_material": self.desc.id,
            "p4kx_shader": self.desc.shader,
            "p4kx_family": self.family,
        }
        for name, value in list(self.desc.parameters.items())[:MAX_PROPERTIES]:
            properties[f"p4kx_param_{name}"] = value
        return properties

    def _nodraw(self) -> GraphSpec:
        bsdf = self.builder.add("ShaderNodeBsdfTransparent", key="bsdf")
        out = self.builder.add("ShaderNodeOutputMaterial", key="output")
        self.builder.link(Ref(bsdf, "BSDF"), out, "Surface")
        self.builder.spec.alpha_mode = "blend"
        return self.builder.spec

    def _entry(self) -> PaletteColours | None:
        palette = self.context.palette
        if palette is None:
            return None
        entry = palette.entry(self.desc.palette)
        if entry is None and self.family == "decal" and self._has_decal_placeholder():
            return palette.primary
        return entry

    def _has_decal_placeholder(self) -> bool:
        return any(s.file.startswith(TINT_PALETTE_DECAL) for s in self.desc.textures)

    def _base(self) -> Surface:
        entry = self._entry()
        factor = entry.tint if entry else self.desc.diffuse
        smooth = entry.glossiness / 255.0 if entry else self.desc.shininess / 255.0
        return self._surface(self.desc, factor, smooth, 1.0, 1.0)

    def _surface(
        self, desc: MaterialDesc, factor: Colour, smooth: float, gloss_mult: float, tiling: float
    ) -> Surface:
        b = self.builder
        colour: Ref | Colour = factor
        if slot := desc.slot("diffuse"):
            texture = Ref(self._image(slot, tiling), "Color")
            colour = texture if factor == WHITE else b.mix(texture, factor, 1.0, blend="MULTIPLY")
        if slot := desc.slot("occlusion"):
            colour = b.mix(colour, Ref(self._image(slot, tiling), "Color"), 1.0, blend="MULTIPLY")
        if slot := desc.slot("detail"):
            colour = b.mix(colour, Ref(self._image(slot, tiling), "Color"), 0.5, blend="OVERLAY")
        normal: Ref | None = None
        smoothness: Ref | float | None = None
        if slot := desc.slot("normal"):
            image = self._image(slot, tiling)
            map_key = b.add("ShaderNodeNormalMap", props={"space": "TANGENT"})
            b.link(Ref(image, "Color"), map_key, "Color")
            normal = Ref(map_key, "Normal")
            if slot.texture and self.context.texture_alpha.get(slot.texture) == "smoothness":
                smoothness = Ref(image, "Alpha")
        if smoothness is None and (slot := desc.slot("gloss")):
            gloss = b.add("ShaderNodeRGBToBW")
            b.link(Ref(self._image(slot, tiling), "Color"), gloss, "Color")
            smoothness = Ref(gloss, "Val")
        if smoothness is None:
            return Surface(colour, min(max(1.0 - smooth * gloss_mult, 0.0), 1.0), normal)
        scaled = smoothness if gloss_mult == 1.0 else b.math("MULTIPLY", smoothness, gloss_mult)
        return Surface(colour, b.math("SUBTRACT", 1.0, scaled), normal)

    def _layer_surface(self, layer: Layer, desc: MaterialDesc) -> Surface:
        palette = self.context.palette
        entry = palette.entry(layer.palette) if palette else None
        tint = layer.tint_colour
        if entry:
            factor = _multiply(entry.tint, tint or WHITE)
            smooth = entry.glossiness / 255.0
        else:
            factor = tint or desc.diffuse
            smooth = desc.shininess / 255.0
        return self._surface(desc, factor, smooth, layer.gloss_mult or 1.0, layer.uv_tiling or 1.0)

    def _layered(self) -> Surface:
        desc = self.desc
        pairs = [
            (layer, self.context.materials[layer.material])
            for layer in desc.layers
            if layer.material in self.context.materials
        ]
        if not pairs:
            return self._base()
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
        wear = self._wear_factor()
        current = foundation
        for channel, (layer, layer_desc) in zip(channels, pairs):
            surface = self._layer_surface(layer, layer_desc)
            if wear is not None and layer.wear_tint:
                surface = surface._replace(colour=self._worn(surface.colour, layer.wear_tint, wear))
            current = self._blend(current, surface, channel)
        return current

    def _blend_channels(self) -> list[Ref]:
        b = self.builder
        if slot := self.desc.slot("blend") or self.desc.slot("mask"):
            source = self._image(slot, 1.0)
        else:
            source = b.add("ShaderNodeVertexColor", props={"layer_name": "Color"})
        split = b.add("ShaderNodeSeparateColor")
        b.link(Ref(source, "Color"), split, "Color")
        alpha_output = "Alpha"
        return [Ref(split, "Red"), Ref(split, "Green"), Ref(split, "Blue"), Ref(source, alpha_output)]

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

    def _blend(self, below: Surface, above: Surface, factor: Ref) -> Surface:
        b = self.builder
        node = b.add("ShaderNodeGroup", group=LAYER_BLEND)
        b.put(node, "Colour A", below.colour)
        b.put(node, "Colour B", above.colour)
        b.put(node, "Roughness A", below.roughness)
        b.put(node, "Roughness B", above.roughness)
        b.put(node, "Factor", factor)
        normal_a, normal_b = below.normal, above.normal
        if normal_a is None and normal_b is None:
            return Surface(Ref(node, "Colour"), Ref(node, "Roughness"), None)
        geometry = b.add("ShaderNodeNewGeometry")
        b.put(node, "Normal A", normal_a or Ref(geometry, "Normal"))
        b.put(node, "Normal B", normal_b or Ref(geometry, "Normal"))
        return Surface(Ref(node, "Colour"), Ref(node, "Roughness"), Ref(node, "Normal"))

    def _image(self, slot: TextureSlot, tiling: float) -> str:
        assert slot.texture is not None
        key = (slot.texture, tiling, slot.tiling, slot.offset)
        cached = self._images.get(key)
        if cached is not None:
            return cached
        b = self.builder
        image = b.add("ShaderNodeTexImage", texture=slot.texture)
        scale = (slot.tiling[0] * tiling, slot.tiling[1] * tiling)
        if scale != (1.0, 1.0) or slot.offset != (0.0, 0.0):
            coordinates = b.add("ShaderNodeTexCoord")
            mapping = b.add(
                "ShaderNodeMapping",
                props={"vector_type": "POINT"},
                inputs={"Location": (*slot.offset, 0.0), "Scale": (*scale, 1.0)},
            )
            b.link(Ref(coordinates, "UV"), mapping, "Vector")
            b.link(Ref(mapping, "Vector"), image, "Vector")
        self._images[key] = image
        return image

    def _alpha(self) -> Ref | float | None:
        desc, b = self.desc, self.builder
        mode = "blend" if self.family in ("glass", "decal", "hologram") else desc.alpha_mode
        source: Ref | None = None
        if slot := desc.slot("mask"):
            bw = b.add("ShaderNodeRGBToBW")
            b.link(Ref(self._image(slot, 1.0), "Color"), bw, "Color")
            source = Ref(bw, "Val")
        elif (
            (slot := desc.slot("diffuse"))
            and slot.texture
            and self.context.texture_alpha.get(slot.texture) == "opacity"
        ):
            source = Ref(self._image(slot, 1.0), "Alpha")
        self.builder.spec.alpha_mode = mode
        if mode == "mask":
            return b.math("GREATER_THAN", source, desc.alpha_test) if source else None
        if mode != "blend":
            return None
        opacity = desc.opacity
        if self.family == "decal" and (extra := desc.number("DecalDiffuseOpacity")) is not None:
            opacity *= extra
        if source is None:
            return opacity if opacity < 1.0 else None
        return source if opacity == 1.0 else b.math("MULTIPLY", source, opacity)

    def _principled(self, surface: Surface) -> None:
        desc, b = self.desc, self.builder
        entry = self._entry()
        bsdf = b.add("ShaderNodeBsdfPrincipled", key="bsdf")
        out = b.add("ShaderNodeOutputMaterial", key="output")
        b.link(Ref(bsdf, "BSDF"), out, "Surface")
        colour = surface.colour
        if self.family == "glass" and desc.palette == "glass" and self.context.palette:
            colour = self.context.palette.glass
        b.put(bsdf, "Base Color", colour)
        b.put(bsdf, "Roughness", surface.roughness)
        if surface.normal is not None:
            b.link(surface.normal, bsdf, "Normal")
        metal = "metal" in desc.surface_type.lower()
        b.spec.node(bsdf).inputs["Metallic"] = 1.0 if metal else 0.0
        specular = entry.specular if entry else desc.specular
        if specular != WHITE:
            b.spec.node(bsdf).inputs["Specular Tint"] = specular
        if slot := desc.slot("specular"):
            bw = b.add("ShaderNodeRGBToBW")
            b.link(Ref(self._image(slot, 1.0), "Color"), bw, "Color")
            b.link(Ref(bw, "Val"), bsdf, "Specular IOR Level")
        if self.family == "glass":
            b.spec.node(bsdf).inputs["Transmission Weight"] = 1.0
            b.spec.node(bsdf).inputs["IOR"] = 1.45
        emissive = desc.emissive
        if desc.glow > 0 and any(c > 0 for c in emissive):
            b.spec.node(bsdf).inputs["Emission Color"] = emissive
            b.spec.node(bsdf).inputs["Emission Strength"] = desc.glow
        if self.family == "hologram":
            b.put(bsdf, "Emission Color", colour)
            b.spec.node(bsdf).inputs["Emission Strength"] = max(desc.glow, 1.0)
        alpha = self._alpha()
        if alpha is not None:
            b.put(bsdf, "Alpha", alpha)


def _multiply(a: Colour, b: Colour) -> Colour:
    return (a[0] * b[0], a[1] * b[1], a[2] * b[2])
