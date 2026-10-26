# Run-time graphics of screens are listed with the field that would name them

Decided for issue #581 (follow-ups of #537), amending ADR-0105 and ADR-0111.

## Decisions

- A widget whose graphic the game picks at run time is listed with the Record field that holds the
  name when the Records name one, so that a later issue can choose documented defaults:
  `WidgetRuntimeImage` names `imageSource`, a custom shape or icon without a path has an empty
  `svgPath` (`svgFill.svgPath` on the other widgets), and the overlay and body background widgets
  have an empty `urlOptional`. The reasons keep their first words, so a filter on them still
  matches.
- Widget names are listed as the Records hold them. The `<unnamed>` of a widget without a name is
  not escaped in the text and in the JSON report; only an SVG document escapes text, in the one
  place that writes it (`p4kx-gfx`). On main the text
  report and the JSON report of the radar master canvas print `<unnamed>`, and a test pins it.
