# An Interior is packaged on its own with the container as root

Decided for issue #553 (follow-up of #161). Builds on ADR-0083 (the package), ADR-0096 (Interiors
in format 1.2), ADR-0107 (the Character entry point) and #136 (`interior glb`).

## Decision

**No format change.** The nodes of an Interior already exist in format 1.2 (`container`, `object`,
`light`). An Interior on its own needs no new kind and no new field, so the package stays at
format 1.3 and the add-on reads it unchanged. Its root is the container's own node, kind
`container`, without a `parent`, at the identity transform; the `.soc` files, nested containers,
objects and lights hang below it as below a container of an Entity's Interior. Their `path` starts
at the root's name; there is no `Interior` group node, because that group exists only to hold the
containers of an Entity.

**`source.entity` is filled as for a Character.** It stays required for every reader of 1.x, so it
carries the file name of the container's Entry without folder and extension as `class_name` and an
empty `guid`: an Interior is not an Entity of the Archive. The add-on names its collection after
it. The chain of Entry paths is in the command's report, not in the manifest.

**One library entry point, shared logic.** `p4kx_blender::write_interior_package` takes an
`InteriorScene` (a name and a built `Interior`), the model provider and texture loader, the GLB
options, the `PackageInfo` and the output folder, next to `write_package` and
`write_character_package`, so the GUI follow-up (#570) calls it without a new code path. It builds
its node list from the `Interior` with the code that builds the nodes below an Entity's container,
and then runs the same staging, output guard, mesh plan, texture writing, replace-on-complete rule
and cancellation as `write_package`; the two entry points differ only in where the node list and
`source.entity` come from.

**The command.** `p4kx interior package <archive> <container>... --out <folder>` takes the container
chain of `interior glb` and `--lod`, `--texture-mip` and `--no-vertex-colours` of `entity package`.
The Interior is built exactly as `interior glb` builds it; the report counts the same things as
`entity package` does, with `container_nodes` for the count of container nodes because `containers`
is the chain, as in `interior glb`. The layer materials and textures are looked up in the Archive
the chain starts in.

## Consequences

- A package of an Interior is a package of format 1.3 like any other; consumers need no change.
- Models that exist only inside a container's own Archive layer keep their geometry (the Interior
  holds it), but a texture that exists only inside a container is not found and is listed as a
  failed texture.
