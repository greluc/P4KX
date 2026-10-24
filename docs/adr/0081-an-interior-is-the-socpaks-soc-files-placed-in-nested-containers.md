# An Interior is the socpak's .soc files placed in nested containers

Decided for issue #136. Evidence: LIVE socpaks of the RSI Mantis, AEGS Hammerhead, Idris and
Javelin (`p4kx interior glb`, 1 s each, 15 to 2097 placed models).

## Decision

A socpak container holds `<stem>.xml`, the description of its object container, and the `.soc`
files it streams. The description lists the `.soc` files in `StreamingOCs/OC@name`, nested
`OC` elements included, and may list containers nested in it in
`ChildObjectContainers/Child` (`name`, `pos`, `rot` as a quaternion `w,x,y,z`, `entityName`,
`class`, `external`). A `.soc` file is a `CrCh` chunk file; its `IncludedObjects` chunk
(type `0x0010`) places the models of the container: a table of 256-byte model paths, then one
208-byte record per placement. Per record: kind `u32` at 0 (1 for a placed model), the index into
the path table as `u16` at 60, the transform as a row-major 3x4 matrix of `f64` at 64, the tint
palette as `u8` at 172. The matrix is in the container's space, Z up, metres; on the Mantis body
all 15 models fall inside the bounds the description declares.

`p4kx-model` reads both (`ObjectContainer::parse`, `IncludedObjects::read`) from bytes.
`p4kx-assembly` has `Interior::build`, which walks the containers, composes the transforms
(parent transform applied to the child's, so a child's position is rotated by the parent), reads
the models like an Assembly (each file once, in parallel, shared `Arc`s so ADR-0077 shares the
meshes) and lists whatever failed as `Issue`s. `Interior::add_to_scene` adds the parts below any
scene node, which is the seam for the Entity Assembly (#138); `InteriorPartKind` is non-exhaustive
for props and lights (#137).

Scene nodes: the container, a `<file>.soc` node per streamed file below it, a node per model
below that, and nested containers below their parent. `.soc` files share the container's space.

## Resolution

A model path is looked for in the container of its `.soc` file, then in the containers around it,
last in the Archive below `Data`. A nested container is looked for by its `name` plus `.socpak`
in the same order, and in the Archive also below `Data` and `Data\ObjectContainers`. A container
whose identity (layer and Entry path) is one of its ancestors is reported as a cycle and not
entered. Limits: 16 levels, 1024 containers, 65536 parts, the vertices of the distinct models
(as for Assemblies), 128 MiB per description or `.soc` file, 1 Mi placements per `.soc`.

## Open points

No LIVE ship socpak examined holds `ChildObjectContainers`; the reader follows the StarBreaker
description and is tested on synthetic data. Record kinds other than 1 (kind 16 occurs in the
Hammerhead and Javelin) have an unknown length, so the list ends there and
`unsupported_objects` is reported. Entities of the `.soc` (props, lights) are #137.
