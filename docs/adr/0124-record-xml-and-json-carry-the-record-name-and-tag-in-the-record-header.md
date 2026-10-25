# Record XML and JSON carry the Record name and tag in the Record header

Decided for issue #285, on top of ADR-0004 (the canonical Record tree) and ADR-0008 (record files).

## Decision

- The Record name (`Record::name`, e.g. `EntityClassDefinition.AEGS_Gladius`) and the Record tag of
  a version 8 DataForge (`Record::tag`, e.g. `SystemsDesign`) are part of the canonical Record tree
  (`RecordTree::name`, `RecordTree::tag`) and of both written forms.
- Record JSON writes them as `$name` and `$tag` in the Record object, after `$path`. A field whose
  name starts with `$` is already written by position, so no field can collide with them. `$name`
  is always there, an empty string for a Record without a name; `$tag` is left out for version 6,
  for a Record without tag and for an empty tag. The JSON Schema keeps `$name` optional so that
  exports from before this change still validate.
- Record XML writes them as the attributes `p4kx:name` and `p4kx:tag` of the root, after `path`
  and before the fields. They are namespaced because plain `name` is a common field name that
  must stay an attribute of its own field. The root of a document now always declares
  `xmlns:p4kx`; a root inside a wrapper element still leaves the declaration to the wrapper.
- An empty tag counts as no tag in the tree, the XML, the JSON and the Comparison, so a version 8
  DataForge whose Records have no tags gives the same output as version 6.
- The Comparison reports the tag next to the name in `old` and `new` of a Record change and
  treats a different tag, like a different name, as a change of the Record. The Comparison window
  lists a changed tag as a row of its own next to the name and type.
- This is additive: no existing key, attribute or order changes, and readers that ignore unknown
  keys and attributes are not affected. Record XML readers that look for a root without namespace
  declarations see `xmlns:p4kx` on records that had none. The Basetool catalog is not affected.
- The flat value export (ADR-0112) carries the tag as an appended ninth column `record_tag`
  (amendment of issue #614).

## Consequences

- Records that share one record path (ADR-0008) can be told apart inside a file by `$name`.
- A consumer that needs one "main" Record per file chooses its own rule; dcb-editor takes the
  first Record per file name, StarBreaker the last. P4KX exports no main Record.
