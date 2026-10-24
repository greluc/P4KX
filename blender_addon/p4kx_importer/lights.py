# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""The mapping from a package light to Blender light settings, without Blender.

The mapping is documented in ``docs/blender-package.md``. Colours are linear and carried over
unchanged. The intensity is the engine's raw value, whose unit is not documented, so the energy
is only a relative, approximate figure: ``WATTS_PER_UNIT`` watts per unit for point and spot
lights, ``SUN_STRENGTH_PER_UNIT`` for sun lights, times the strength the user chooses.
"""

from __future__ import annotations

import math
from dataclasses import dataclass

from .package import Light, Quaternion, gltf_to_blender_quaternion

WATTS_PER_UNIT = 10.0
SUN_STRENGTH_PER_UNIT = 1.0
MIN_SPOT_SIZE = math.radians(1.0)
MAX_SPOT_SIZE = math.radians(180.0)

_BLENDER_TYPES = {"point": "POINT", "spot": "SPOT", "directional": "SUN"}
_QUARTER_TURN_ABOUT_X: Quaternion = (math.sqrt(0.5), math.sqrt(0.5), 0.0, 0.0)


@dataclass(frozen=True, slots=True)
class LightSettings:
    """The values to set on a Blender light: ``bpy.types.Light`` attribute names."""

    type: str
    color: tuple[float, float, float]
    energy: float
    use_custom_distance: bool
    cutoff_distance: float
    spot_size: float | None
    spot_blend: float | None


def light_settings(light: Light, strength: float = 1.0) -> LightSettings:
    """Returns the Blender settings for ``light``, its energy multiplied by ``strength``.

    A spot light's cone is the full angle ``2 * outer``, within Blender's 1 to 180 degrees; its
    blend is ``1 - inner / outer``, as Blender's glTF importer maps it. A range becomes the
    custom distance, a light without one has no cut-off.
    """
    per_unit = SUN_STRENGTH_PER_UNIT if light.kind == "directional" else WATTS_PER_UNIT
    spot_size = None
    spot_blend = None
    if light.kind == "spot" and light.outer_cone is not None:
        spot_size = min(max(2.0 * light.outer_cone, MIN_SPOT_SIZE), MAX_SPOT_SIZE)
        inner = light.inner_cone or 0.0
        spot_blend = min(max(1.0 - inner / light.outer_cone, 0.0), 1.0) if light.outer_cone else 0.0
    return LightSettings(
        type=_BLENDER_TYPES[light.kind],
        color=light.colour,
        energy=light.intensity * per_unit * max(strength, 0.0),
        use_custom_distance=light.range is not None,
        cutoff_distance=light.range if light.range is not None else 0.0,
        spot_size=spot_size,
        spot_blend=spot_blend,
    )


def gltf_to_blender_light_quaternion(rotation: Quaternion) -> Quaternion:
    """Converts the rotation of a package light node to the ``(w, x, y, z)`` of a Blender light.

    A package light shines along its local -Z axis in glTF's Y-up axes; Blender's lights shine
    along their local -Z axis in Z-up axes. The conversion that carries the node's axes to
    Blender's maps that direction to +Y, so the light is turned a quarter turn about X.
    """
    return _multiply(gltf_to_blender_quaternion(rotation), _QUARTER_TURN_ABOUT_X)


def _multiply(a: Quaternion, b: Quaternion) -> Quaternion:
    aw, ax, ay, az = a
    bw, bx, by, bz = b
    return (
        aw * bw - ax * bx - ay * by - az * bz,
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
    )
