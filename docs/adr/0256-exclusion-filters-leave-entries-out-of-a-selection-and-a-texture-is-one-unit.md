# Exclusion Filters leave Entries out of a selection, and a texture is one unit

Decided for issue #857. Extends the Filter of CONTEXT.md.

## Decision

- An exclusion Filter is a Filter with the same rules (glob over the full path when it holds `*`
  or `?`, otherwise a substring; ASCII case and separator kind do not matter). There is no second
  matcher: `Exclusion` holds `Filter`s and asks each of them.
- An Entry is selected when the include Filter matches it and no exclusion Filter does.
  `p4kx extract --filter F --exclude X …` and `p4kx search F --exclude X …` take the option
  repeatedly; `--exclude` without `--filter` is a usage error. The Extraction dialog of the GUI
  takes one exclusion Filter per line for the Entries of the Archive; the changes of a Comparison
  side are not narrowed there.
- Exclusion Filters are structural: they match Entry paths only.
- A Filter pair that selects nothing is invalid input (exit 2). The error says which side removed
  everything: `no Entry matches <F>` when the include Filter matched none, `the exclusion Filters
  leave out all <n> Entries that <F> matches` when the exclusion removed them. `p4kx search`
  keeps an empty result a result unless `--exclude` is given.
- A split texture is one unit. When the header (`.dds`) matches an exclusion Filter, its parts
  `.dds.N` and alpha tail `.dds.a`, `.dds.Na` are left out with it, so a Conversion does not
  write them as stray files. When only a part matches, the part is left out of the written
  Entries, and a Conversion still reads it from the Archive to build the texture. That is
  reported rather than dropped silently: a note on stderr of `p4kx extract --convert`, and a
  line of the summary of the GUI Extraction. Without a Conversion the part is simply not written
  and nothing is reported.
- Filter text from outside is bounded by `Filter::parse`: at most 1024 bytes and 32 wildcards per
  Filter, and `Exclusion::parse` takes at most 64 Filters and refuses an empty or blank one,
  which would leave out every Entry or every Entry with a space in its path. The GUI takes one
  Filter per line, ignores empty lines and refuses lines of spaces; its field takes at most four
  times the Filter limit in characters and grows with its text inside the scrolling dialog. Matching is a single left-to-right pass that resumes only at the most
  recent `*`, so its work per path is at most the product of the path and Filter lengths; no
  regular expression is built and nothing backtracks over earlier wildcards. `p4kx extract`,
  `p4kx search`, `p4kx socpak extract` and the GUI use `parse`; the other callers keep `new`.

## Consequences

- The rule that decides what an exclusion leaves of a texture exists once
  (`Exclusion::leave_out`), generic over the item, so the CLI works on `&Entry` and the GUI on
  Entry ids with the same code.
- The rule works on paths alone, without looking up the header, so a part whose header is not in
  the Archive counts as a part: a Filter that matches the path its header would have leaves it
  out, one that matches only the part leaves it out and counts it in the report although no
  texture stays.
