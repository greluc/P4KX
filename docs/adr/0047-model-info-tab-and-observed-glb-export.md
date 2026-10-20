# The model preview lists a model in an Info tab; Export as GLB writes through an observed stream

Decided for issue #109.

## Decision

- The model preview reads every level of detail of the model once and keeps two results: the
  `Scene` of level 0 for the 3D view and a small `ModelInfo` with the counts and notes the Info
  tab lists. The info exists whenever the model could be read, also when the view cannot show it
  (a skeleton file, a model without triangles). A model whose other levels cannot be read falls
  back to level 0 alone, so the view keeps working and the info lists the levels that were read.
- The Info tab is a tab of its own next to Model, laid out like the DataForge summary: the facts as
  key and value rows, then the levels of detail and the chunks that were not read as tables. The
  texts are formatted and laid out once per model, language, width and scale and kept in egui's
  frame data, so a frame allocates nothing for them.
- Export as GLB reads the model again for the chosen level only, on its own thread, and writes
  through `ModelSource::write_glb_observed`. The stream reports the bytes written against the length
  the GLB header states and fails with a marker once the caller's `CancelToken` is set; the
  temporary file next to the target is then removed. Cancelling before the first write returns
  `ConversionError::Cancelled` without creating anything.
- The summary of an export lists the chunks of the exported files that were skipped or unknown and
  the materials that fell back to the default material. Missing textures join it when textures are
  exported (#105).
- The choices (level, materials, vertex colours, folder) are kept in the settings file as
  `model_export`; a level the next model lacks is replaced by its most detailed one.
- Animations are not read yet (#107); the Info tab says so instead of showing zero.

## Considered Options

- Count the chunks per level in the info from a second read of level 0 only: the info would not
  list the other levels, which the dialog needs for its choice.
- Cancel by dropping the thread: the file would be left half written; the cancel token with a
  failing writer removes the temporary file through the existing atomic write.
- Show the info as a panel beside the 3D view: it would take width from a view that is already
  short (D.35); a tab keeps the view as it is.
