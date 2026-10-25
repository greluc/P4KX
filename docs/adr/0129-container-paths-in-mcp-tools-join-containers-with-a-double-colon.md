# Container paths in the MCP tools join containers with a double colon, and a preview returns an image block

Decided for issue #286. Builds on ADR-0038 and ADR-0023.

The MCP server read only the Entries of the Archive itself. The rooms, props and materials of an
Entity live in socpak containers inside it, so a client could not look into them, and it had no way
to see a texture or to resolve a Loadout.

## Decision

- **A container path is `outer.socpak::inner.socpak::path`.** `::` cannot occur in an Entry path of
  a Windows-style Archive, so it separates the containers from the path inside the last one. Every
  segment before the last names a container Entry, matched ignoring case and separator kind like any
  Entry. `list_entries` takes it as `folder` (`outer.socpak::` is the top level), `search_entries`
  as `container`, and `read_entry_text`, `read_entry_bytes`, `image_preview` and `mtl_summary` as
  `path`. An empty container name is an error.
- **Listings return paths a client can pass on unchanged.** An Entry or folder inside a container is
  shown as the Entry path of its containers, `::` and its own path.
- **Containers stay open per Archive.** The last four opened containers of an Archive are kept with
  their folder tree and path index, and are dropped with the Archive. The limits of
  `Archive::open_container` (depth 8, 512 MiB decoded, entry count, compression ratio) apply.
- **`image_preview` returns the image as a PNG image content block** plus structured metadata, not
  only a path, because the client reads the image from the result and the server writes no file. It
  is bounded: at most 2048x2048 pixels, 128 MiB of texture data, 1 MiB of PNG and two decodes at a
  time. The description carries the ADR-0038 sentence that media must not be redistributed.
- **Budgets:** `entity_loadout` lists ports in pages bounded by the 256 KiB response cap and the
  page size, at most 32 levels deep (16 by default); texts are cut at 2048 characters and lists
  inside one material at 64 items. No tool runs code or changes state.

## Considered Options

- **A separate `container` argument on every tool:** rejected for the read tools, since one path
  string is what a listing returns and what a client copies. `search_entries` has no path, so it
  takes `container`.
- **`!` or `|` as the separator:** rejected. `!` is used by Java-style jar paths but is legal in
  Entry names; `::` is not found in any Archive path and reads clearly.
- **Return only the temporary path of a written PNG:** rejected. The server never writes and the
  client may run on another machine.

## Consequences

- A client reaches every file of a socpak without extracting it.
- The tools return pixels of game textures to the client, which may forward them; the notice in
  each description and in the server instructions says so.
