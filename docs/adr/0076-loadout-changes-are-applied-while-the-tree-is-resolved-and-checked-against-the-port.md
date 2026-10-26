# Loadout changes are applied while the tree is resolved and checked against the port

Decided for issue #128, which builds on the Loadout tree of ADR-0012 and the Assembly of
ADR-0040.

## Decision

Exclusions and replacements are `LoadoutChanges`, handed to `LoadoutResolver::resolve_with`. They
change the tree while it is built, not afterwards, so `entity loadout`, `entity glb` and every
front-end see one tree and the Assembly needs no knowledge of them:

- A port is named by its path of port names from the root joined by `/`, ignoring ASCII case:
  the path `EntityNode::walk` lists. A path that matches no port is an error, so a typo does not
  silently export a ship with its weapons.
- An excluded port stays in the tree with the source `excluded` and holds nothing. Its subtree is
  not resolved, so nothing below it is exported, and it is no Assembly issue. The prefetch of
  Records does not know the changes and may still read what lies below it. An exclusion below
  another is redundant and dropped.
- A replaced port holds the replacing Entity with the source `replaced`. The Entity brings its
  own default Loadout; the Loadout the old entry gave the replaced Entity belongs to that Entity
  and is discarded (ADR-0012 applies to the new Entity as to any). Ports below a replacement
  can be changed too, by their path in the new subtree.
- The port keeps its definition, so the replacement is placed at the port's helper like the
  Entity before it.
- Contradicting changes are errors: a port both excluded and replaced, a replacement inside an
  excluded branch, a port replaced twice.
- A replacement by the root Entity or by an Entity the port lies below would put the tree inside
  itself, and is an error (`ChangeError::Cycle`).
- Where docked vehicles are not expanded (`entity glb` by default), the ports below one are not
  in the tree, so a change to such a port, as `entity loadout` lists it, is not applied and no error. The resolver returns such paths (`ChangedLoadout::ignored`) and
  the front-ends warn about them on stderr and list them as `ignored` in the JSON.

## Compatibility

A replacement is checked against what the port takes (`PortNode::constraints`): the item types
with their sub-types and the range of sizes, compared with the Entity's `AttachDef.Type`,
`SubType` and `Size` (`ItemFacts`). The constraints come from the port's `SItemPortDef` when the
Record defines the port, else from the item port of the vehicle implementation (`minSize`,
`maxSize` and `Types`), which is where most ship hardpoints are declared. Types and sub-types are
compared ignoring ASCII case; a port that lists no types takes any, an Entity without an item type
does not fit a port that lists some, and an Entity that gives no size is not held against the
sizes. An Entity whose sub-type is empty or `UNDEFINED` fits any sub-type a port lists, as the
default quantum and jump drives of the ships do, which carry `UNDEFINED` on ports that list
`QDrive` and `JumpDrive`; both kinds of port share this rule. A port with no constraints at all
takes any Entity. The rejection carries the reason
(`Incompatibility`) and is invalid input in the CLI (exit code 2).

Tags (`PortTags`, `RequiredPortTags`, `AttachDef.Tags`) are not checked: they are References to
Tag Records whose use in the game is not settled here. That is a narrowing step that can follow
without changing the API. The `subtypes` of a vehicle implementation's `Types/Type` are read and
checked like those of a port definition.

## Not decided here

A hardpoint that only the vehicle implementation declares and no Loadout entry names is not a
port of the tree, so nothing can be attached to it; the tree would have to list it first.
