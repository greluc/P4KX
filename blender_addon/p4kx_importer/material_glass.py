# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""The glass family of the material mapping: Base Color and alpha (ADR-0100, ADR-0253, ADR-0258)."""

from __future__ import annotations

from .graph_spec import GraphBuilder, Ref
from .material_model import WHITE, Colour, MaterialDesc
from .material_surface import (
    GLASS_CLEAR_ALPHA,
    GLTF_COLOUR_ATTRIBUTE,
    PER_VERTEX_TINT,
    TINT_STRENGTH,
    BuildContext,
    luminance,
)


class GlassSurface:
    """Mixin of the mapping: the parts that only glass has."""

    desc: MaterialDesc
    context: BuildContext
    builder: GraphBuilder

    def _glass_base_colour(self, colour: Ref | Colour) -> Ref | Colour:
        """Returns the Base Color of glass: the palette's glass colour if named, then the tint.

        The palette colour is faded towards white by ``GlassTintStrength`` (default 1). With
        ``PER_VERTEX_TINT`` the colour is multiplied by the vertex colours.
        """
        desc, b = self.desc, self.builder
        if desc.palette == "glass" and self.context.palette:
            given = desc.number(TINT_STRENGTH)
            strength = 1.0 if given is None else min(max(given, 0.0), 1.0)
            colour = self.context.palette.glass
            if strength < 1.0:
                r, g, bl = colour
                colour = (1.0 - strength * (1.0 - r), 1.0 - strength * (1.0 - g), 1.0 - strength * (1.0 - bl))
        if PER_VERTEX_TINT not in desc.gen_mask:
            return colour
        node = b.add("ShaderNodeVertexColor", props={"layer_name": GLTF_COLOUR_ATTRIBUTE})
        vertex = Ref(node, "Color")
        return vertex if colour == WHITE else b.mix(vertex, colour, 1.0, blend="MULTIPLY")

    def _glass_alpha(self, colour: Ref | Colour, opacity: Ref | float | None) -> Ref | float:
        """Returns the alpha of glass: ``1 - (1 - GLASS_CLEAR_ALPHA) * luminance(colour)``.

        EEVEE draws nothing behind a blended surface through Transmission, so the alpha lets
        the scene through; the darker the tint, the more of the glass shows. Cycles stays
        clear because its Transmission already does the same. ``opacity`` scales the result.
        """
        b = self.builder
        spread = 1.0 - GLASS_CLEAR_ALPHA
        alpha: Ref | float
        if isinstance(colour, Ref):
            grey = b.add("ShaderNodeRGBToBW")
            b.link(colour, grey, "Color")
            alpha = b.math("SUBTRACT", 1.0, b.math("MULTIPLY", Ref(grey, "Val"), spread), clamp=True)
        else:
            alpha = 1.0 - spread * luminance(colour)
        if opacity is None:
            return alpha
        if isinstance(alpha, float) and isinstance(opacity, float):
            return alpha * opacity
        return b.math("MULTIPLY", opacity, alpha, clamp=True)


__all__ = ["GlassSurface"]
