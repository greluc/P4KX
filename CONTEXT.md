# P4K-Extractor

Opens, browses, compares and extracts the game archives of Star Citizen and turns their internal
formats into readable or importable data for people and for other KRT tools.

## Language

### The archive

**Channel**:
One installed release line of Star Citizen (LIVE, PTU, EPTU, HOTFIX, TECH-PREVIEW), each with its
own Archive.
_Avoid_: branch, environment, build

**Library folder**:
A folder holding Channel folders, e.g. `C:\Program Files\Roberts Space Industries\StarCitizen`; the
RSI Launcher installs every Channel into one.
_Avoid_: install folder, game folder

**Manual entry**:
A Channel folder, Library folder or Archive the user added by hand, searched in addition to the
detected places.
_Avoid_: custom path, bookmark

**Recent Archive**:
One of the last Archives opened in the window, the latest first.
_Avoid_: history, MRU

**Build**:
The game version an Archive belongs to, as stated in the Channel's build manifest (branch, version,
change number).
_Avoid_: patch, release, version (alone)

**Archive**:
The `Data.p4k` file of one Channel, holding every game file as an Entry.
_Avoid_: pak, package, zip

**Entry**:
One file inside an Archive, identified by its path in the Archive.
_Avoid_: file, asset, member

**Filter**:
Text that selects Entries by path: a glob over the full path when it holds `*` or `?`, otherwise
a substring anywhere in the path; case and separator kind do not matter.
_Avoid_: query (a Query selects Records), search pattern

**Layout**:
The on-disk structure of an Archive's directory: v1 (ZIP64 central directory with local headers)
or v2 (fixed 204-byte records and a name table behind a trailer ending in `Jiji`).
_Avoid_: format version, Archive version

**Preview**:
The start of one Entry shown in the window as text or as hexadecimal bytes, read without an
Extraction.
_Avoid_: viewer, inspector

### Game data

**DataForge**:
The game's typed object database, stored in the Entry `Data\Game.dcb` (or `Game2.dcb`).
_Avoid_: DataCore, dcb, Game database

**Record**:
One typed object in DataForge, identified by its GUID and addressed by its record path.
_Avoid_: dcb entry, item (an item is one kind of Record)

**Record path**:
The file a Record is defined in, without `.xml`, e.g. `libs/foundry/records/entities/spaceships/aegs_avenger`;
several Records may share one, so only the GUID is unique.
_Avoid_: file name, record name (the name is `EntityClassDefinition.AEGS_Avenger`)

**Record folder**:
A record path that several Records share, shown in the Record tree as a folder holding those Records.
_Avoid_: record group, file node

**Record tag**:
A label DataForge version 8 stores with every Record, e.g. `SystemsDesign` or `Unknown`; version 6
Records have none.
_Avoid_: category, kind

**Reference**:
A link from one Record to another by GUID; it is never expanded into the referring Record.
_Avoid_: weak pointer, pointer (alone)

**Query**:
A path expression that selects Records or values inside Records.
_Avoid_: search (searching is over Entry paths), filter (a Filter selects Entries)

**CryXml**:
The binary XML encoding CryEngine uses for many Entries outside DataForge.
_Avoid_: CryXmlB, binary XML

### Assets

**Entity**:
A Record that describes a placeable game object such as a ship, vehicle, weapon or item.
_Avoid_: object, actor

**Loadout**:
The tree of Entities attached to an Entity's ports, e.g. a ship's components and weapons.
_Avoid_: fitting, equipment

**Assembly**:
An Entity combined with its Loadout, geometry, materials and textures into one exportable model.
_Avoid_: scene, build (a Build is a game version)

**Interior**:
The rooms and props of an Entity, stored in socpak containers inside the Archive.
_Avoid_: room, level

**WEM**:
A Wwise sound file, loose as an Entry or embedded in a soundbank, holding one audio stream in one
codec (PCM, IMA ADPCM, Wwise Vorbis, Opus, PTADPCM, …).
_Avoid_: audio file, sound file

**Character**:
A human body with head (CHF/DNA), armour and clothing, assembled like an Entity.
_Avoid_: avatar, player model

### Operations

**Extraction**:
Writing selected Entries or Records from an Archive into a folder on disk.
_Avoid_: unpack, export (an Export is something else)

**Conflict policy**:
What an Extraction does with a file already in the target folder: skip it, overwrite it, or
overwrite it only if changed (size or CRC-32C differs from the Entry or the converted Record).
_Avoid_: collision mode, overwrite mode

**Conversion**:
Turning an Entry or Record from its game-internal format into a readable one, e.g. CryXml or a
Record into XML.
_Avoid_: decoding, transformation

**Comparison**:
The set of Entries and Records that were added, removed or changed between two Archives.
_Avoid_: diff (fine in code), delta

**Release**:
A published version of P4K-Extractor itself: a `v*` tag with its portable ZIP, its MSI and its
notes from `CHANGELOG.md`.
_Avoid_: build (a Build is a game version), update

**Update check**:
The optional, user-switched question to GitHub whether a newer Release than the running one exists;
it only links to that Release, never downloads it.
_Avoid_: updater, auto-update

**Basetool catalog**:
The JSON Export of manufacturers, items, ships, commodities and blueprints that the basetool's P4K
import consumes.
_Avoid_: p4k-catalog, basetool export

**Kind**:
One group of entities in the Basetool catalog, e.g. manufacturers or items, selected by record-path
prefix and type and complete or partial on its own.
_Avoid_: category, entity type

**Export**:
A structured file derived from DataForge for another tool, as opposed to an Extraction of Entries.
_Avoid_: dump

**Record export**:
The generic Export of selected Records as Record XML or JSON, one file per Record or one combined
file.
_Avoid_: JSON dump, DataForge dump
