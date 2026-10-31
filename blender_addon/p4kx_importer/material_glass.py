# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""The glass family of the material mapping: Base Color and tinted transparency (ADR-0258, ADR-0282)."""

from __future__ import annotations

from .graph_spec import GraphBuilder, Ref
from .material_model import WHITE, Colour, MaterialDesc
from .material_surface import (
    GLASS_BSDF_SHARE,
    GLTF_COLOUR_ATTRIBUTE,
    TINT_STRENGTH,
    BuildContext,
    reads_vertex_colours,
)


class GlassSurface:
    """Mixin of the mapping: the parts that only glass has."""

    desc: MaterialDesc
    context: BuildContext
    builder: GraphBuilder

    def _glass_base_colour(self, colour: Ref | Colour) -> Ref | Colour:
        """Returns the Base Color of glass: the palette's glass colour if named, then the tint.

        The palette colour is faded towards white by ``GlassTintStrength`` (default 1). Glass that
        reads the vertex colours (``PER_VERTEX_TINT``) then multiplies the colour by them.
        """
        desc, b = self.desc, self.builder
        if desc.palette == "glass" and self.context.palette:
            given = desc.number(TINT_STRENGTH)
            strength = 1.0 if given is None else min(max(given, 0.0), 1.0)
            colour = self.context.palette.glass
            if strength < 1.0:
                r, g, bl = colour
                colour = (1.0 - strength * (1.0 - r), 1.0 - strength * (1.0 - g), 1.0 - strength * (1.0 - bl))
        if not reads_vertex_colours(desc):
            return colour
        node = b.add("ShaderNodeVertexColor", props={"layer_name": GLTF_COLOUR_ATTRIBUTE})
        vertex = Ref(node, "Color")
        return vertex if colour == WHITE else b.mix(vertex, colour, 1.0, blend="MULTIPLY")

    def _glass_surface(self, bsdf: str, colour: Ref | Colour, opacity: Ref | float | None) -> Ref:
        """Returns the glass shader: a tinted transparent surface with a share of the Principled BSDF.

        EEVEE draws nothing behind a blended surface through Transmission, but a Transparent BSDF
        multiplies what is behind it by its colour, as the game's glass colour does. The mix is
        ``GLASS_BSDF_SHARE`` of ``bsdf`` and the rest transparent in ``colour``, so dark glass
        dims the scene behind it instead of hiding it. ``opacity`` fades the result to clear.
        """
        b = self.builder
        tint = b.add("ShaderNodeBsdfTransparent")
        b.put(tint, "Color", colour)
        glass = b.add("ShaderNodeMixShader", inputs={"Fac": GLASS_BSDF_SHARE})
        b.link(Ref(tint, "BSDF"), glass, 1)
        b.link(Ref(bsdf, "BSDF"), glass, 2)
        if opacity is None:
            return Ref(glass, "Shader")
        clear = b.add("ShaderNodeBsdfTransparent")
        faded = b.add("ShaderNodeMixShader")
        b.put(faded, "Fac", opacity)
        b.link(Ref(clear, "BSDF"), faded, 1)
        b.link(Ref(glass, "Shader"), faded, 2)
        return Ref(faded, "Shader")


__all__ = ["GlassSurface"]
