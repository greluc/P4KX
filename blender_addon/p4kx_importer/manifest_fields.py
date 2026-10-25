# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""The error type and the field readers every part of the manifest reader shares."""

from __future__ import annotations

from typing import Any

MAX_ITEMS = 200_000

Vector3 = tuple[float, float, float]
Quaternion = tuple[float, float, float, float]


class PackageError(Exception):
    """The package cannot be read: it is malformed or not a P4KX package."""


class PackageTooNewError(PackageError):
    """The package has a higher major version than this add-on reads."""


def read_object(value: object, where: str) -> dict[str, Any]:
    """Returns ``value`` when it is an object, else raises ``PackageError``."""
    if not isinstance(value, dict):
        raise PackageError(f"{where} must be an object.")
    return value


def read_array(value: object, where: str) -> list[Any]:
    """Returns ``value`` when it is an array of at most ``MAX_ITEMS`` entries."""
    if not isinstance(value, list):
        raise PackageError(f"{where} must be an array.")
    if len(value) > (limit := MAX_ITEMS):
        raise PackageError(f"{where} has more than {limit} entries.")
    return value


def read_text(entry: dict[str, Any], key: str, where: str) -> str:
    """Returns the string ``entry[key]``, else raises ``PackageError``."""
    value = entry.get(key)
    if not isinstance(value, str):
        raise PackageError(f"{where}: {key} must be a string.")
    return value
