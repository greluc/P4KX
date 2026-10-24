# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""The light record of a package node and its validation, without Blender."""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Any

MAX_INTENSITY = 1e9
LIGHT_KINDS = frozenset({"point", "spot", "directional"})
QUARTER_TURN = math.pi / 2 + 1e-6


class LightError(ValueError):
    """A light record is malformed."""


@dataclass(frozen=True, slots=True)
class Light:
    """A punctual light: linear colour, the engine's raw intensity, range and cone angles."""

    kind: str
    colour: tuple[float, float, float]
    intensity: float
    range: float | None
    inner_cone: float | None
    outer_cone: float | None


def parse_light(value: object, where: str) -> Light | None:
    """Returns the light, or ``None`` for a light kind a later minor version added.

    Raises ``LightError`` for a record that is not an object or has invalid values.
    """
    if not isinstance(value, dict):
        raise LightError(f"{where} must be an object.")
    kind = value.get("kind")
    if not isinstance(kind, str):
        raise LightError(f"{where}: kind must be a string.")
    if kind not in LIGHT_KINDS:
        return None
    colour = value.get("colour")
    if not isinstance(colour, list) or len(colour) != 3:
        raise LightError(f"{where}: colour must be 3 finite numbers.")
    channels = tuple(_number(channel, f"{where}.colour") for channel in colour)
    if not all(0.0 <= channel <= 1.0 for channel in channels):
        raise LightError(f"{where}: colour must lie within 0 and 1.")
    intensity = _number(value.get("intensity"), f"{where}.intensity")
    if not 0.0 <= intensity <= MAX_INTENSITY:
        raise LightError(f"{where}: intensity must lie within 0 and {MAX_INTENSITY:g}.")
    light_range = _optional(value, "range", where)
    if light_range is not None and light_range <= 0.0:
        raise LightError(f"{where}: range must be positive.")
    inner = _optional(value, "inner_cone", where)
    outer = _optional(value, "outer_cone", where)
    if kind == "spot":
        if inner is None or outer is None or not 0.0 <= inner < outer <= QUARTER_TURN:
            raise LightError(f"{where}: a spot light needs 0 <= inner_cone < outer_cone <= pi/2.")
    else:
        inner = outer = None
    return Light(kind, (channels[0], channels[1], channels[2]), intensity, light_range, inner, outer)


def _number(value: Any, where: str) -> float:
    if not isinstance(value, (int, float)) or isinstance(value, bool):
        raise LightError(f"{where} must be a finite number.")
    try:
        number = float(value)
    except OverflowError as error:
        raise LightError(f"{where} must be a finite number.") from error
    if not math.isfinite(number):
        raise LightError(f"{where} must be a finite number.")
    return number


def _optional(entry: dict[str, Any], key: str, where: str) -> float | None:
    return None if entry.get(key) is None else _number(entry[key], f"{where}.{key}")
