# Hash functions of CHF fields

A CHF names almost everything by a 32-bit hash. Two functions are in use, and they are not
interchangeable. The helpers live in `p4kx_character` (`names.rs`).

| Field | Function | Input | Helper |
|---|---|---|---|
| DNA magic (`DNA_MAGIC`, `0xFCD09394`) | CRC-32C | `dna matrix 1.0` | `name_hash` |
| `Dna::gender_hash` | CRC-32C | rig name `protos_human_<male\|female>_face_t1_pu`; male `0xDD6C67F6`, female `0x9EF4EB54` | `name_hash` |
| `Dna::variant_hash` | unknown | the low half of the rig's identity | none |
| `ItemPort::port` | CRC-32C | lower-cased port name, e.g. `head_itemport` | `lowered_hash` |
| `Material::attachment` | CRC-32 (IEEE) | lower-cased port name followed by `_skinitemport` | `skin_attachment` |
| `SubMaterial::name` | CRC-32C | lower-cased sub-material name, e.g. `shader_eyeInner` | `lowered_hash` |
| `FloatParam::name` | CRC-32C | exact parameter name, e.g. `DyeAmount` | `name_hash` |
| `ColorParam::name` | CRC-32C | exact parameter name when known | `name_hash` |

- **CRC-32C** is the Castagnoli polynomial (`crc32c` crate). **CRC-32 IEEE** is the zlib
  polynomial (`crc32fast` crate). The container checksum of the file is CRC-32C too.
- The slot formula in numbers: `head_itemport` gives `0xA98BEB34`, `body_itemport` `0x27424D58`,
  `eyes_itemport` `0xA047885E`, `hair_itemport` (the hair dye slot) `0x6C836947`, `beard_itemport`
  `0x9B274D93`, `eyebrow_itemport` `0x078AC8BD`, `stubble_itemport` `0xA8770416`.
- A few colour parameters are named by hashes that are the CRC of no known name: `BODY_COLOR`
  (`0xBD530797`), `EYE_COLOR` (`0x442A34AC`), `HAIR_DYE_COLOR` and `HAIR_TINT_COLOR`. They are
  constants in `names.rs`, found by comparing the shipped Characters. Some HairPBR parameter
  hashes of other tools are mapped by hand for the same reason.
- Face animations and the DNA rig use CRC-32 IEEE too, but for other things: the controller ID of
  a track is the hash of a bone name (`docs/model-animations.md`), and the control hashes of the
  RigLogic chunk are the hash of the control name.
- Tests: the unit tests in `names.rs` pin the DNA magic, both gender hashes and the slot
  formula. The ignored real-Archive test `real_chf_hashes` resolves the attachment hashes of every
  CHF of the Archive against the known item port names and prints the share that does not. On
  the LIVE Archive of build 4.10.2 (41 CHF files) all 260 attachment hashes resolve by the CRC-32
  formula, and 306 of 322 item port hashes match a known name by CRC-32C; the other 16 are two
  distinct hashes, `0x6D6DE693` and `0x6958D171`, whose names are not known.

## Names in the JSON

p4kx chf to-json writes the name of every hash it can resolve beside it, in a *_label key
that rom-json ignores (ADR-0178). NameTable in p4kx_character holds the names per hash
function: item ports by lowered_hash, the attachments of materials by skin_attachment of the
port name, sub-materials by lowered_hash, parameters by 
ame_hash, and the two rigs. The
built-in table covers the names found by analysing the Characters; p4kx_assembly::archive_names
adds the Archive's: the ports of the Entity of every worn item, and the sub-materials and public
parameters of the base material libraries the materials choose. On the LIVE Archive of build 4.10.2
(41 CHF files, 4345 hashes) the built-in table names 1043 hashes and the Archive names 2855; 64
distinct hashes stay unresolved, among them the hair dye and tint colour parameters. The ignored
real-Archive test eal_chf_names measures this and checks the byte-exact round trip of every
labelled JSON.
