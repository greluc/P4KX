### Removed

- Record writers: `RecordXml::write_with`, `RecordJson::write_with`,
  `RecordXml::write_element_with` and `RecordXml::write_wrapped_element_with` in `p4kx-dataforge`
  (also re-exported by `p4kx-extraction`). Call `write_annotated(tree, texts.into(), out)`,
  `write_element_annotated(tree, texts.into(), depth, out)` or
  `write_wrapped_element_annotated(tree, texts.into(), depth, out)` instead; `texts` is the same
  `Option<&dyn TextResolver>` and the output is unchanged.
