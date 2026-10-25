# Container paths in the MCP tools join containers with a double colon, and a preview returns an image block

Decided for issue #286. Builds on ADR-0038 and ADR-0023.

The MCP server read only the Entries of the Archive itself. The rooms, props and materials of an
Entity live in socpak containers inside it, so a client could not look into them, and it had no way
to see a texture or to resolve a Loadout.

## Decision

- **A container path is `outer.socpak::inner.socpak::path`.** `::` does not occur in
  real Archives, so it separates the containers from the path inside the last one; an Entry that
  has it in its path is unreachable through container paths. Every
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
  is bounded: at most 2048x2048 pixels (counting every image of the mip it decodes), 128 MiB of
  texture data, 1 MiB of PNG and two decodes at a time. The description carries the ADR-0038 sentence that media must not be redistributed.
- **Budgets:** `entity_loadout` and `mtl_summary` list in pages bounded by the page size and by
  the 256 KiB cap on the whole response, the fields around the list included. Ports are at most 32
  levels deep (16 by default). Texts are cut at 256 characters (a port path at 1024); the lists
  inside one item are capped, with the full count beside them: 64 texture slots, shader parameters,
  layers and Entity issues, 16 item types per port and 16 sub-types per item type. The caps keep
  one item far below the response cap, so no item blocks paging. No tool runs code or changes state.

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

## Amendment (issue #622)

- **The kept containers have a byte budget.** At most four containers and 256 MiB are kept per
  Archive, each charged its decoded size plus 256 bytes per Entry for its path index and folder
  tree; the least recently used are dropped first. A container charged more than 128 MiB is not
  kept: the calls that opened it use it and it is dropped with them. Two Archives are kept, so at
  most 512 MiB of containers are held.
- **A container is decoded outside every lock and under a model permit.** Calls that need a
  container being decoded wait for that decode on their blocking thread and share its result; if
  it fails, each waiting call tries again. The permit is the one of the chunk file, model and
  texture reads (two at a time), and a call takes it only on a blocking thread that holds its
  worker permit, always in the order worker permit, then model permit. `image_preview` therefore
  takes the permit after its path is resolved, and the model tools inside their call, so that no
  call waits for a worker while holding a permit a decode needs.
- **A path inside a container is found through its path index**, built once per container, not by
  a scan of its Entries per segment.
- **`entity_loadout` resolves a tree once.** The resolved tree of an Entity is kept per Archive,
  keyed by the Entity as written and `stop_at_docked_vehicles`; the last four are kept and are
  dropped with the Archive, which is opened anew when its file changes. Paging, and a changed
  `max_depth`, walk the kept tree. A page still costs one walk of the tree, but no resolving.
  Calls that miss at the same time each resolve, and the first tree kept wins.
