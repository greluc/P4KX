# The Record writers take Annotations only, and the GUI export and preview show tag paths

ADR-0052 added `write_annotated` to the writers of Record XML and Record JSON and kept the older
`write_with`, so two entry points did the same with different arguments, and `write_element_with`
and `write_wrapped_element_with` doubled the element writers the same way.

- **One entry point per output.** `RecordXml` and `RecordJson` write through `write_annotated`
  (`write_element_annotated` and `write_wrapped_element_annotated` for the element forms) with an
  `Annotations`; `write` and `write_element` remain as the plain forms. `write_with`,
  `write_element_with` and `write_wrapped_element_with` are removed. A caller with an
  `Option<&dyn TextResolver>` passes `texts.into()`. The output bytes do not change.
- **Record export option.** The GUI Record export offers "write the tag path next to every tag
  Reference", off by default, as `p4kx dcb export --tags` does. It applies to JSON and NDJSON; the
  flat forms have no place for it, so the checkbox is disabled for them and the request drops it.
  The tag tree comes from the cache of the Archive's DataForge, built once and shared with the Tags
  tab; a cancel during that build ends the export as cancelled with nothing written.
- **Record preview.** A tag Reference shows its tag path, muted, where other References show the
  type of their target. The worker builds the tag tree on the first tag Reference it meets, not
  when the preview opens, and the Record XML tab stays plain.
- **Small window.** At 640 × 400 the work surface keeps its minimum width and scrolls sideways
  (`work_scroll`); the copy buttons of the Tags tab lie
  inside that surface, which a test checks. The window layout itself stays with #358.
