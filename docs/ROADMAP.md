# Roadmap

P4KX opens, browses, compares and extracts Star Citizen Archives and converts their
formats. It replaces the KRT P4K Reader and covers the functionality of unp4k and StarBreaker with
an own Rust core (ADR-0001). Terms follow [CONTEXT.md](../CONTEXT.md).

Every milestone ends in a usable release: SemVer `0.x`, one minor version per milestone, built by
GitHub Actions from a `v*` tag as a portable ZIP with `p4kx-gui.exe`, `p4kx.exe` and
`p4kx-mcp.exe` and a per-user MSI (ADR-0007), with notes from `CHANGELOG.md`.

## Product decisions

| Topic | Decision |
|---|---|
| Purpose | End-user tool first; the core is a library so Exports for other KRT tools need no rework |
| Front-ends | `p4kx` (CLI), `p4kx-gui` ("P4KX", egui/eframe), `p4kx-mcp` (MCP server, stdio, read-only) |
| Platforms | Windows is shipped; the code stays platform-neutral and CI also builds on Linux; no macOS |
| Distribution | Portable ZIP and MSI (WiX Toolset 7, per-user install to `%LOCALAPPDATA%`, no admin rights, Start menu entry, stable UpgradeCode) from M1 |
| Licence | `MIT OR Apache-2.0`; the repository becomes public later |
| UI languages | German (`de-DE`) and British English (`en-GB`) via Fluent, chosen from the system locale, switchable |
| Look | KRT design system (colours, Lato) in egui, dark only (ADR-0006); native window frame tinted via DWM |
| State | `%APPDATA%\P4KX\` (TOML): detected Channels, recent Archives, target folder, language, window |
| Channel detection | Library folders named in the RSI Launcher log (ADR-0005), the default install path, and manually added folders and Archives; `loginData.json` and the launcher's settings store are never read |
| Network | Only an optional, switchable check for new GitHub releases, off by default (ADR-0007); no telemetry, no crash reports |
| Live changes | Archives are opened read-only with shared access; a changed size, timestamp or Build cancels running jobs and offers a reload |

## Milestones

### M1 — Archive, browsing, Extraction, CLI

- Read Archive layouts v1 and v2 (`Jiji` trailer): ZIP64 central directory, Store / Deflate / Zstd
  (method 100), AES-128-CBC encrypted Entries with a key the user supplies (ADR-0027); positional
  reads from a handle pool.
- Read the Channel's Build from `build_manifest.id`.
- GUI: Channel picker, drag and drop, path tree, sortable Entry table, preview (text, hex), search
  with globs and substrings, quick-filter chips, status bar.
- Extraction of selected, shown or all Entries, cancellable with progress; default target is a new
  folder `SC-EXPORT-<Channel>-<Build>-<timestamp>`, an existing folder asks skip / overwrite / only
  if changed; paths outside the target are rejected.
- CLI `p4kx list | search | extract` with JSON output and documented exit codes.
- Workspace, lints, layer and file-size checks, CI, release workflow with EXE and MSI.

### M2 — CryXml and DataForge

- CryXml to XML.
- DataForge versions 6 and 8 (`Game2.dcb`, falling back to `Game.dcb`) into the canonical Record
  tree; owned children nested, References as links (ADR-0004).
- Record XML, one file per Record under its record path, optionally one combined file.
- GUI: DataForge mounted as a virtual Record tree, XML and DataForge preview tabs.

### M3 — Exports (replaces the KRT P4K Reader)

- Basetool catalog in the new format (ADR-0002): manufacturers, items, ships, commodities,
  blueprints; Channel, Build and completeness per kind; JSON Schema generated from the Rust types
  and checked in.
- Record export as one JSON file per Record or one NDJSON file, from the same Record tree.
- Hand-off: a separate session adapts the basetool import in `krt-profit/basetool`.

### M4 — Comparison

- Entries added, removed and changed (size and CRC from the central directory, no decompression).
- Records added, removed and changed, field by field on the Record tree.
- Line diff for XML and text Entries.
- GUI view and a JSON report.

### M5 — Textures and localisation

- DDS: merge split mips (`.dds.1` … `.dds.N`, cubemaps, alpha tails), decode BC1–BC7 and
  uncompressed formats, reconstruct normal maps; preview and PNG export.
- Localisation: `global.ini` per language, `LOC_` keys resolved in previews and Exports, searchable
  table.

### M6 — Audio

- WEM: Vorbis, PCM, ADPCM and Opus detection; export as Ogg or WAV, chosen at Extraction.
- BNK soundbanks: embedded WEMs; playback in the preview.

### M7 — Models

- CryEngine chunk files (`#ivo`, `CrCh`): meshes, LODs, skeletons, animations (CAF, DBA).
- glTF / GLB export with materials (`.mtl`) and textures as PNG, skeletons and animations.

### M8 — Queries and MCP

- Query language with dot paths and component selectors, in GUI and CLI.
- `p4kx-mcp`: read-only tools for Archive, DataForge, Queries and chunk files.

### M9 — Assembly and 3D viewer

- Assemblies of Entities with their Loadout (ships, vehicles, weapons) as one GLB with LOD, mip and
  material options.
- GUI 3D viewer (wgpu): orbit, LOD and material switch, skeleton, animation playback.

### M10 — Interiors

- socpak containers placed into the Assembly of their Entity.

### M11 — Wwise resolution

- Entity to audio trigger to soundbank to WEM, including streamed and localised sources; sounds per
  Entity in the GUI.

### M12 — Characters and armour

- CHF and DNA heads, body, armour and clothing as Characters; face and body animation.

### M13 — Blender package

- Decomposed package (`scene.json`, GLB parts, textures) and a Python importer add-on in
  `blender_addon/`.

### M14 — MFD and holograms

- Ship screens (GFx/SWF) and vehicle holograms rendered to images.

## Engineering

- Workspace of `p4kx-*` crates in three layers with enforced direction (ADR-0003, ADR-0019):
  - formats: `foundation`, `archive`, `cryxml`, `dataforge`, `localization`, `texture`, `audio`,
    `chunks`, `model`, `character`, `gfx`
  - services: `assembly`, `gltf`, `blender`, `comparison`, `export`, `extraction`
  - front-ends: `p4kx`, `p4kx-gui`, `p4kx-mcp`
- Libraries are synchronous and parallelise with rayon; tokio only in `p4kx-mcp`.
- Rust edition 2024, `rustfmt`, clippy `all`, `pedantic`, `nursery` and selected `restriction`
  lints as errors, `missing_docs`; suppressions only as `#[expect(lint, reason = "…")]`.
- `unsafe_code = "forbid"` everywhere except the `cfg(windows)` DWM module in `p4kx-gui`.
- `thiserror` in libraries, `anyhow` only in binaries; no `unwrap` / `expect` outside tests.
- Source files stay below about 400 lines; clippy `too_many_lines` and `cognitive_complexity`.
- Tests: synthetic fixtures from own test writers (Archive, DataForge, CryXml, DDS / WEM headers)
  run in CI; tests against real Archives run locally when `P4KX_DATA_P4K` points to one.
- CI on Windows and Linux: `fmt`, `clippy`, `test`, layer-direction and file-size checks.
- References: StarBreaker and unp4k (both MIT) and the observed behaviour of the KRT P4K Reader;
  ported code keeps its copyright notice.
