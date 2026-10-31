# An Entity places its object containers at its bones

Decided for issue #138, which builds on ADR-0081 (the Interior). Evidence: LIVE Constellation
Andromeda (5 containers, 86 parts, 69 models) and Cutlass Black (3 containers), whose models
all lie inside the hull's bounds with this rule and 9 of 69 and 7 of 10 outside without it.

## Decision

**Where the containers come from.** An Entity Record names its socpak containers in two places:
`VehicleComponentParams.objectContainers[]` (`fileName`, `boneName`, `Offset` as a position and an
`Ang3` rotation in degrees) and `SObjectContainerComponentParams.objectContainer` (a file only).
`EntityNode::object_containers` lists both, the vehicle's first, as `ObjectContainerRef`s. Only
the root Entity's containers are used; containers of attached Entities are not yet.

**How they are placed.** A container sits at the node of the hull's model named by `boneName`
(the root part's model). The Record's offset is in the Entity's space, so it normally equals the
bone: composing both doubled every position (the Constellation's front section at 39.5 m instead
of 19.75 m). A census of every Entity with containers on LIVE (1326 Entities, 5048 containers)
found 191 containers without a bone name, 45 whose bone the hull lacks, 996 with an offset of
zero, 3555 whose offset equals the bone within 0.05 m and 261 that disagree. The disagreements
are of two kinds: a bone at the hull's origin with the real position in the offset (MISC Hull C,
Command Module), and hulls longer or shorter than the base ship the offset was written for (the
Constellation Phoenix and Taurus, XIAN Scout, Retaliator animated parts, differences of 0.05 m to
several metres). So the rule is, comparing translations only (0.05 m per axis):

- a bone that resolves and sits away from the origin places the container alone; an offset that
  disagrees with it is an `offset_differs` issue (an offset of zero is no disagreement);
- a bone at the origin places the container with the offset: `bone * offset`;
- no bone name, or a bone the hull lacks (`helper_not_found`), places it at the offset from the
  origin, whatever its size;
- an offset that is not finite is the identity with an `invalid_offset` issue.

The offset rotation uses the convention of port offsets (X, then Y, then Z). The real-Archive
tests check that every placed container of the Constellation and the Cutlass equals this rule and
that all its models lie inside the hull's bounds.

**API.** `EntityInterior::build(archive, assembly, tree, options)` opens each container from the
Archive (below `Data` or as spelled), builds its `Interior` with `Interior::build`, and returns the
placed containers and the issues. `Assembly::with_interior` adds it: the Assembly's GLB then has a
node `Interior` below the root Entity's node, a node per container below it (ADR-0081's nodes
inside), and the Interior's issues join `Assembly::issues`. Without `with_interior` the Assembly is
the hull alone. The CLI switch is `p4kx entity glb --interior`; a front-end needs only the two
calls. The Interior is built after the Assembly and does not change its parts, so a viewer can
add or drop it without rebuilding the hull.

**Unresolved parts.** A container the Archive lacks or cannot open is `container_missing` or
`container_unreadable` with path `Interior`; what an Interior cannot resolve keeps its kind, with
the container's name put in front of its path. At most `MAX_INTERIOR_CONTAINERS` containers are
opened per Entity.

**Limits per Entity.** `InteriorOptions` gains `max_containers`. The limits of the options hold for
the Entity as a whole: each container is built with what is left of the containers, parts and
vertices after the containers before it, and the hull's own vertices are counted. When one is
used up the remaining containers are left out with `too_many_containers`, `too_many_parts` or
`too_many_vertices`.

## Verification

`InteriorFit::new(assembly, tree, interior)` (feature `verification` of `p4kx-assembly`) measures
each container in the world: the box around the static models its `.soc` files place (models
that are not `.cga`; props such as doors, lockers and cabinets do not bound the room), the
functional hardpoints of the Assembly inside and outside it, and how far its animated parts
(`.cga` models: doors, ramps, drawers) lie from it. Hardpoints are told by the item type of the
Entity at the port, never by name: a `FlightController`, a `Seat`, a `CargoGrid`, a `Door` or a
`DockingAnimator`. The pilot seat is the seat of the same vehicle nearest to the flight
controller.

**Static models, room models, hull.** A hardpoint or an animated part is measured against the box
of a single static model of a container, never against the union box of all of them
(`ContainerFit::models`, `holds`; the union stays available as `room()`). A `.soc` may place the
model of a neighbouring hull section next to its own (the Cutlass cockpit socpak also places
`int_body`), so the union can be much longer than the room. Every static model counts, because a
census of 15 ships on LIVE (Caterpillar, 890 Jump, Hull C, Starfarer, Carrack, Constellation
Andromeda and Aquila, Cutlass, Polaris, Idris, Valkyrie, Corsair, Freelancer, Starlifter C2,
Redeemer) found cockpit and bridge seats inside models with a box volume of only 0.003 to 0.24 of
their container's largest model (890 Jump battle bridge 0.029, Idris rear bridge 0.003, Valkyrie
pilot seat 0.24), so no volume threshold separates rooms from props. A container whose static
models are all unreadable has none and holds nothing.

The hull rule is the one that may ignore small models, since dropping one can only weaken it. Its
*room models* are the static models with at least `ROOM_VOLUME_SHARE` (a quarter) of the largest
one's box volume (a container whose models are all flat has zero volume everywhere and keeps all
of them). In the census the smallest room model has 0.251 and the largest dropped one
0.248, so the quarter is a cut in a continuum; it keeps the Cutlass cockpit model (0.31) and
raises no false alarm. Taking all models instead gives false alarms from stray models of the
Carrack and the Idris beyond the hull.

`InteriorFit::misfits` applies three rules (`tolerance` is how far a hardpoint may lie outside a
model box; the Constellation and Cutlass tests use 0.5 m, the census 1 m):

- the flight controller of the root vehicle lies in a container, within the tolerance of a static
  model (`ControllerOutsideRooms`; a docked ship's flight controller sits at that ship's origin,
  below its cockpit, so only the root vehicle's is checked);
- when some cargo grid lies inside a static model, every cargo grid lies within the tolerance of
  one (`CargoGridOutsideRooms`); a ship none of whose grids lies inside a model carries its cargo
  outside any room and is not checked;
- the room models of each container of the root Entity lie within `HULL_SLACK` (1 m) of the box of
  the root Entity's model (`RoomOutsideHull`); containers of attached Entities, such as the
  Caterpillar command module, lie outside the hull by design and are left out. In the census no
  root container protrudes at all.

The Constellation and Cutlass tests also keep every door and every animated part within the
slack of a model. The census test asserts that no rule fires on any of the 15 census ships that
the Archive holds (the 600i is not found).

**Census of the controller and cargo rules (LIVE, 15 ships).** The first version of these rules
(the controller and its pilot seat in one container; every cargo grid in a container without
cockpit hardpoints) held only on the Constellation and the Cutlass. The measurements that
replaced them:

- Root controllers lie 0.00 to 0.13 m from a static model on 13 ships, 0.29 m on the Starlifter C2
  (at the origin) and 0.87 m on the Starfarer (outside its bridge container). The rule therefore
  needs a tolerance of 1 m on the census; the margin to the Starfarer is 0.13 m, a cut in a
  continuum.
- A controller does not always sit in the cockpit room, so the controller and the seat nearest to
  it are not a pair: the Starfarer and Starlifter C2 pair it with a copilot seat, the Carrack with
  a drone seat. The pair rule is dropped, and no rule names a pilot seat, so `pilot_seats` is
  removed.
- Controller rooms also hold cargo on the Caterpillar, 890 Jump, Carrack, Idris and Redeemer, so
  a cargo room free of cockpit hardpoints does not exist there. The rule of a separate cargo room
  is dropped.
- Cargo grids lie 0.00 to 0.06 m from a static model on 14 ships and inside one on every one of
  them. The Hull C's 16 strut grids lie 0.96 to 4.25 m from every model and none inside: its
  cargo is carried on open struts. Hence the all-or-nothing rule above: a ship with any grid
  inside a model is checked for all its grids, a ship with none is not. A single misplaced cargo
  room of such a ship, or of one with one grid, is therefore only caught as far as the hull rule
  catches it.
- Doors and animated parts (Idris, Starlifter C2, Caterpillar command module, Carrack cargo pods,
  Corsair, Polaris and others, 0.5 to 10.6 m from every model of their container, or in a
  container with no static model) break any rule on the census and are checked on the
  Constellation and the Cutlass only.

**Limit.** `InteriorFit::new_displaced` moves one container in the world. Moved 3 m forwards, the
Cutlass cockpit container still holds the controller and every seat in the box of all its
models, but its cockpit model then protrudes about 3 m beyond the hull and the hull rule fires.
Moved 3 m backwards it is not detected: the 8 m long cockpit model still holds the controller and
the copilot seat, and the pilot seat leaves it by 0.65 m, which a rule for every seat of the root
vehicle would catch but which also fails on the Redeemer's escape-pod bedding (0.57 m from any
model).

Identifying the pilot seat by structure, to check only it, was tried on the census and found not
reliable: the nearest seat to the controller is the Cutlass copilot seat; the `SCItemSeatParams`
and dashboard parameters of the Cutlass pilot and copilot seat Records are the same
(`seatType`, control template, engine and weapon interactions); the vehicle definition's
`Camera type="pilot"` offsets exist only on the Cutlass of the 12 census ships whose definitions were read
(and its z is not the seat's); the thruster and flight-controller ports carry no controller tag
to match a seat against; and the `controllableTags` of the seat ports are free-form names
(`pilotSeat`, `Pilot_Seat`, `ConsolePilotSeat`, none on the Caterpillar). So no pilot seat rule
is applied.

Two other rules tried on the census were dropped: a connected group of abutting
containers (real ships have gaps of 1 to 11 m between sections) and containers' volume ratio as
the room test (above). Synthetic ships show the checks fail for a cockpit room moved away and for
a controller whose room moved away and for a cargo grid that lies in no room while another does;
a cargo grid in the cockpit room is no misfit.

## Open points

The 0.05 m tolerance and the origin test are heuristics of the Records seen on LIVE. For the 261
disagreeing containers the bone is trusted over the offset, which is right for the longer Phoenix
hull and unverified for the others.
Containers in the Records of
attached Entities (modules) are not placed. Models of the Interior are drawn with plain colours or
the textures of the materials, without the hull's tint palette.
