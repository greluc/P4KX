# Shop components are read in every container by anchor, and placeholder inventories are no list

Decided for issue #481. Amends ADR-0071; builds on ADR-0054 and ADR-0069.

ADR-0071 gave a shop its item list from the inventory file that the `.soc` of its container names.
Measured on LIVE, that rule saw only part of what the files say, and it opened only the shop
folders. This ADR keeps the rule (every place that names the Entity must name the same existing
file) and widens what counts as a place.

## Decision

- **The `.soc` stores each string once.** A container file is a pool of strings in order of first
  use. The component name `EntityComponentShop` and the parameter type `SShopComponentParams`
  occur once per file, in its first shop component; the property name `offlineInventoryJSON` too.
  A later component of the same file shows only its new strings: object name, position, the
  `CryGUID` of its Entity if that text was not stored earlier, and the inventory path if no
  earlier component named it. The `CryGUID` text before the component name is therefore the
  Entity's own only sometimes; in the rest-stop and landing-service containers it is the class
  GUID `2845a83c-...` that every `SCShop` shares. That, and not a differing GUID in
  `entitycomponentshop.xml`, is why 144 shops were unscanned.
- **Anchors.** The component is tied to its Entity by two identifiers the container itself
  states: the `CryGUID`s of its `entitycomponentshop.xml`, and the names of the `SCShop` objects
  in its `<container>_editor.xml` (names that are unique in the container). An inventory path
  (the value after an `offlineInventory*` name, or a path in a `ShopInventories` folder) belongs
  to the closest anchor before it, provided only the component name (at most once), the parameter
  type, `offlineInventory*` names, positions, `SCShop`, `PropertiesDataCore`, `__type`,
  `__version`, `__fullyInstanced`, `CastShadowMinSpec`, `AllowBakedRendering`, `RenderProxy` and at
  most one further GUID stand between them within twelve strings. A path with anything else in
  between stays unattributed and its Entity unknown, never guessed. A component name behind an
  anchor with no path found names no file.
- **Every container.** The `.soc` of every socpak below `PU` and `EA` is read, not only the shop
  folders; kind and brand keep coming from the shop folders alone (`kind` stays `null` for a shop
  that only another container names).
- **Prefabs.** `Data\Prefabs\shops\*.xml` name inventory files through `offlineInventory*`
  attributes of `EntityComponentShop` on `Object Type="Entity"`. The editor `Id`
  `{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}` is the container `CryGUID` with the bytes reordered
  (`CCCCBBBB-AAAA-AAAA-` and `DDDDEEEEEEEEEEEE` reversed); the prefab Entities map to the
  `CryGUID`s of containers (8 of 9 checked) and of registry shops. A prefab is one more place per
  Entity.
- **Placeholders are no list.** `temp_outpost_shopinventory` (six commodities: waste, etam, maze,
  neon, processed food, medical supplies) is named for 264 shops of food courts, refineries,
  clothing stores, showrooms and an expo hall alike, and `inv_cryastro_dummy` (three GUIDs no
  Record has) for 136: they are designer placeholders and say nothing of what a shop sells. A shop
  that names one gets `inventory` and `itemGuids` `null`; `ShopSummary.inventory_placeholder`
  counts them. The files are `temp_outpost_shopinventory` and `inv_cryastro_dummy`, in code.
- **Not silent.** New summary fields: `other_containers_scanned`, `other_containers_unreadable`,
  `other_containers_with_shop`, `prefabs_listed`, `prefabs_unreadable`, `prefab_entities`,
  `inventory_placeholder`, `inventories_named`, `inventories_in_archive`. The catalog shape does
  not change.
- **Not done, on purpose.** The `ShopID` of a file and the file names (`inv_<shop>_<place>`) are
  not used: see the evidence.

## Evidence

LIVE, Build 4.10.197, 2026-10-24.

- 9,668 socpaks; 345 `.soc` files hold the component name; 335 containers have an
  `entitycomponentshop.xml` (413 `CryGUID`s), 164 of them in the shop folders (317 opened for
  kind), 171 elsewhere (8,224 opened). The 12 other containers with a component have no
  `entitycomponentshop.xml` (rest-stop exteriors) and stay unlinked. No socpak holds a socpak.
- Of the 190 inventory paths in those `.soc` files, 175 follow an anchor through the allowed
  strings; 15 do not (prefab-baked Entities whose GUID and name strings were stored earlier);
  they stay unknown.
- 119 inventory JSON files (and one empty `Inv_Shopping_Test_FeatureTest.json`, counted as
  unreadable where named). 59 are named by a container or prefab, 60 by none: the 19 anniversary
  sales, `landingservices_a18|grimhex|levski|rs_full_0002-0008`, the `mining_*` files, the other
  city and station shop files. Nothing in the Archive names them: no `.soc`, no prefab, no Record
  (a search for the name of one over Records, texts and text entries finds nothing). They have no
  reliable link to a shop.
- The `ShopID` chain: 87 files have one. For the 44 unnamed of them it equals a registry key in 3
  cases and ends in a registry `entityGuid` in 16, among them clearly foreign ones
  (`inv_dumpersdepot_levski` ends in `SCShop_ShipComponents_GrimHex`). It stays unusable.
- Result: 310 shops get a list from 33 files (Stanton 290, Pyro 20); before, 331 shops, of which
  173 had the placeholder, so 158 real lists. 400 shops name a placeholder, 28 are ambiguous
  (containers name different files), 53 are unscanned (144 before), 1 names the empty file, none
  names a missing one.
- 2,395 of 2,422 item GUIDs (distinct inventories counted once) are an entry of `items`, `ships`,
  `commodities`, `paints` or `loadouts`. The 27 others are no Record of the DataForge at all
  (checked per GUID): all 21 of `inv_commex_tdd_area18` and 6 of 41 of `inv_astroarmada_area18`.
  They are item and commodity definitions that the game removed; the lists keep them, a consumer
  finds no entry for them.

## Consequences

- The coverage stays at 26 % of the shops (310 of 1,184); the cities' shops are mostly in the 60
  files that nothing names. A further step needs a source the Archive does not appear to hold or a
  name rule that the owner accepts as a guess.
- A shop that names a placeholder looks like a shop without a list to the Basetool, as intended.
