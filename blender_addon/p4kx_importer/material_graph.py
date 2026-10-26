# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Maps a material description to a node graph (ADR-0100), without Blender.

The mapping is a pure function of the description, the layer descriptions it names, the mesh's
palette and the alpha meaning of the textures, so all of it is tested without Blender.
"""

from __future__ import annotations

from .graph_groups import LAYER_BLEND, WEAR
from .graph_spec import GraphBuilder, GraphSpec, Ref
from .package import BLENDER_NAME_BYTES
from .material_model import WHITE, Colour, Layer, MaterialDesc, PaletteColours, TextureSlot
from .material_surface import (
    MASK_PRECEDENCE_SHADERS,
    MASKED_LAYERS,
    MAX_BLENDED_LAYERS,
    MAX_PROPERTIES,
    METAL_FROM,
    METAL_TO,
    NEUTRAL_SMOOTHNESS,
    NEUTRAL_SMOOTHNESS_SHADERS,
    PARAM_PREFIX,
    SPECULAR_LEVEL_PER_F0,
    TINT_PALETTE_DECAL,
    BuildContext,
    Surface,
    family_of,
    luminance,
    metal_share,
    multiply,
)

__all__ = [
    "METAL_FROM",
    "METAL_TO",
    "SPECULAR_LEVEL_PER_F0",
    "BuildContext",
    "Surface",
    "build_graph",
    "family_of",
    "metal_share",
]


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
        skipped = 0
        for name, value in list(self.desc.parameters.items())[:MAX_PROPERTIES]:
            key = PARAM_PREFIX + name
            if len(key.encode("utf-8")) > BLENDER_NAME_BYTES:
                skipped += 1
            else:
                properties[key] = value
        if skipped:
            self.builder.spec.warnings.append(
                f"Material {self.desc.name or self.desc.id!r} has {skipped} shader parameter(s) "
                "with a name too long to keep as a custom property."
            )
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
        if entry:
            smooth = entry.glossiness / 255.0
        elif self.desc.shader.lower() in NEUTRAL_SMOOTHNESS_SHADERS:
            smooth = NEUTRAL_SMOOTHNESS
        else:
            smooth = self.desc.shininess / 255.0
        specular = entry.specular if entry else self.desc.specular
        return self._surface(self.desc, factor, smooth, specular, 1.0, 1.0, entry is not None)

    def _surface(
        self,
        desc: MaterialDesc,
        factor: Colour,
        smooth: float,
        specular: Colour,
        gloss_mult: float,
        tiling: float,
        painted: bool,
    ) -> Surface:
        b = self.builder
        diffuse: Ref | Colour = factor
        if slot := desc.slot("diffuse"):
            texture = Ref(self._image(slot, tiling), "Color")
            diffuse = texture if factor == WHITE else b.mix(texture, factor, 1.0, blend="MULTIPLY")
        if slot := desc.slot("occlusion"):
            diffuse = b.mix(diffuse, Ref(self._image(slot, tiling), "Color"), 1.0, blend="MULTIPLY")
        if slot := desc.slot("detail"):
            split = b.add("ShaderNodeSeparateColor")
            b.link(Ref(self._image(slot, tiling), "Color"), split, "Color")
            diffuse = b.mix(diffuse, Ref(split, "Red"), 0.5, blend="OVERLAY")
        colour, metallic, level = self._metal(desc, diffuse, specular, tiling, painted)
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
            roughness = min(max(1.0 - smooth * gloss_mult, 0.0), 1.0)
            return Surface(colour, roughness, normal, metallic, level)
        scaled = smoothness if gloss_mult == 1.0 else b.math("MULTIPLY", smoothness, gloss_mult)
        return Surface(colour, b.math("SUBTRACT", 1.0, scaled, clamp=True), normal, metallic, level)

    def _metal(
        self,
        desc: MaterialDesc,
        diffuse: Ref | Colour,
        specular: Colour,
        tiling: float,
        painted: bool,
    ) -> tuple[Ref | Colour, Ref | float, Ref | None]:
        b = self.builder
        if slot := desc.slot("specular"):
            texture = Ref(self._image(slot, tiling), "Color")
            grey = b.add("ShaderNodeRGBToBW")
            b.link(texture, grey, "Color")
            texel = Ref(grey, "Val")
            if painted:
                scale = SPECULAR_LEVEL_PER_F0 * luminance(specular)
                colour, share = self._by_share(diffuse, specular)
                return colour, share, b.math("MULTIPLY", texel, scale, clamp=True)
            ramp = b.add("ShaderNodeMapRange", props={"data_type": "FLOAT", "clamp": True})
            b.put(ramp, "Value", texel)
            b.put(ramp, "From Min", METAL_FROM)
            b.put(ramp, "From Max", METAL_TO)
            metallic = Ref(ramp, "Result")
            level = b.math("MULTIPLY", texel, SPECULAR_LEVEL_PER_F0, clamp=True)
            return b.mix(diffuse, texture, metallic), metallic, level
        colour, share = self._by_share(diffuse, specular)
        return colour, share, None

    def _by_share(self, diffuse: Ref | Colour, specular: Colour) -> tuple[Ref | Colour, float]:
        share = metal_share(specular)
        if share == 0.0:
            return diffuse, 0.0
        if share == 1.0:
            return specular, 1.0
        return self.builder.mix(diffuse, specular, share), share

    def _layer_surface(self, layer: Layer, desc: MaterialDesc) -> Surface:
        palette = self.context.palette
        entry = palette.entry(layer.palette) if palette else None
        tint = layer.tint_colour
        if entry:
            factor = multiply(entry.tint, tint or WHITE)
            smooth = entry.glossiness / 255.0
            specular = entry.specular
        else:
            factor = tint or desc.diffuse
            smooth = NEUTRAL_SMOOTHNESS
            specular = desc.specular
        painted = entry is not None or (tint is not None and tint != WHITE)
        return self._surface(
            desc, factor, smooth, specular, layer.gloss_mult or 1.0, layer.uv_tiling or 1.0, painted
        )

    def _layered(self) -> Surface:
        desc = self.desc
        pairs = [
            (layer, self.context.materials[layer.material])
            for layer in desc.layers
            if layer.material in self.context.materials
        ]
        if not pairs:
            return self._base()
        if desc.shader.strip().lower() in MASK_PRECEDENCE_SHADERS and (
            desc.slot("blend") or desc.slot("mask")
        ):
            return self._by_precedence(pairs[:MASKED_LAYERS])
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
        wear: Ref | float | None = None
        wear_read = False
        current = foundation
        for channel, (layer, layer_desc) in zip(channels, pairs):
            surface = self._layer_surface(layer, layer_desc)
            if layer.wear_tint:
                if not wear_read:
                    wear, wear_read = self._wear_factor(), True
                if wear is not None:
                    surface = surface._replace(
                        colour=self._worn(surface.colour, layer.wear_tint, wear)
                    )
            current = self._blend(current, surface, channel)
        return current

    def _by_precedence(self, pairs: list[tuple[Layer, MaterialDesc]]) -> Surface:
        surfaces = [self._layer_surface(layer, layer_desc) for layer, layer_desc in pairs]
        if len(surfaces) == 1:
            return surfaces[0]
        channels = self._blend_channels()
        current = surfaces[-1]
        for at in range(len(surfaces) - 2, -1, -1):
            current = self._blend(current, surfaces[at], channels[at])
        return current

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

    def _blend(self, below: Surface, above: Surface, factor: Ref) -> Surface:
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

    def _image(self, slot: TextureSlot, tiling: float) -> str:
        key = (slot.texture or "", tiling, slot.tiling, slot.offset)
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
        self.builder.spec.alpha_mode = mode
        if mode not in ("mask", "blend"):
            return None
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
        b.put(bsdf, "Metallic", surface.metallic)
        if surface.level is not None:
            b.link(surface.level, bsdf, "Specular IOR Level")
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
