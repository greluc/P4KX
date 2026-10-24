# SPDX-License-Identifier: MIT OR Apache-2.0
# SPDX-FileCopyrightText: 2026 the P4KX contributors

"""Packs the add-on into the zip Blender installs as an extension.

Usage: ``python build_zip.py <output folder>``. The zip holds the files of ``p4kx_importer`` at
its root, in a fixed order with fixed timestamps, so equal sources give an equal zip.
"""

from __future__ import annotations

import sys
import tomllib
import zipfile
from pathlib import Path

SOURCE = Path(__file__).resolve().parent / "p4kx_importer"
FIXED_TIME = (2026, 1, 1, 0, 0, 0)


def addon_files(source: Path = SOURCE) -> list[Path]:
    """Returns the files that belong in the zip, sorted, without caches."""
    return sorted(
        path
        for path in source.rglob("*")
        if path.is_file() and "__pycache__" not in path.parts and path.suffix != ".pyc"
    )


def build_zip(output_folder: Path, source: Path = SOURCE) -> Path:
    """Writes ``p4kx_importer-<version>.zip`` into ``output_folder`` and returns its path."""
    manifest = tomllib.loads((source / "blender_manifest.toml").read_text(encoding="utf-8"))
    output_folder.mkdir(parents=True, exist_ok=True)
    target = output_folder / f"{manifest['id']}-{manifest['version']}.zip"
    with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED) as archive:
        for path in addon_files(source):
            info = zipfile.ZipInfo(path.relative_to(source).as_posix(), FIXED_TIME)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            archive.writestr(info, path.read_bytes())
    return target


def main(arguments: list[str]) -> int:
    """Builds the zip into the folder named by the first argument."""
    if len(arguments) != 1:
        print("usage: build_zip.py <output folder>", file=sys.stderr)
        return 2
    print(build_zip(Path(arguments[0])))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
