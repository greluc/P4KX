# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""The description of a material that has none: format 1.0 packages and unpaired entries."""

from __future__ import annotations

from .material_model import BLACK, DIELECTRIC, WHITE, MaterialDesc, TextureSlot
from .package import MeshMaterial, Package

BASIC_USAGES = frozenset({"diffuse", "normal", "specular"})


def basic_description(described: MeshMaterial, package: Package) -> MaterialDesc:
    """Returns a neutral ``Illum`` description with the textures written for ``described``.

    Only diffuse, normal and specular textures the package knows are used. The material blends
    when its diffuse texture carries opacity in its alpha channel, as a described one does.
    """
    slots = tuple(
        TextureSlot(slot=role, file="", usage=role, texture=texture_id)
        for role, texture_id in described.textures.items()
        if role in BASIC_USAGES and texture_id in package.textures
    )
    diffuse = next((s for s in slots if s.usage == "diffuse"), None)
    opacity = diffuse is not None and package.textures[diffuse.texture or ""].alpha == "opacity"
    return MaterialDesc(
        id=f"basic{described.index}",
        name=described.name,
        shader="Illum",
        gen_mask=frozenset(),
        surface_type="",
        two_sided=False,
        no_draw=False,
        diffuse=WHITE,
        specular=DIELECTRIC,
        emissive=BLACK,
        shininess=0.0,
        opacity=1.0,
        alpha_test=0.0,
        glow=0.0,
        alpha_mode="blend" if opacity else "opaque",
        palette=None,
        textures=slots,
    )
