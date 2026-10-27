# The work surface fits the smallest window

Decided for issue #358. Desktop-spec decision D.59, which replaces the sizes of D.52.

## Context

D.52 kept the room of a work tab at 720 by 480 points and scrolled it in a smaller window. At the
smallest window of 640 by 400 the sidebar sits behind a rail, so the work surface was wider than
the window and its main actions, among them "Search", "Copy text", "Copy GUID" and "Copy tag
path", could only be reached by scrolling sideways.

The issue also asked for a 35 px facts strip, an 86 px ghost button and a disabled border of
about `#494a4d`. Those values come from the former external design system, which P4KX no longer
follows: the theme and `docs/design/` are the only reference.

## Decision

- The room of a work tab keeps 580 by 536 points (`workSurface.minWidth`, `minHeight`): the
  smallest window less the 40 point rail and the scrollbar a low window shows. A window of 640
  points scrolls the work surface vertically only.
- The rail is 40 points wide, as `sidebar.railWidth` says; the sidebar collapses below 880 points.
- A toolbar that does not fit 580 points puts its actions in a line of their own, right-aligned,
  and that line wraps when it is still too narrow. The Entry table shrinks the initial widths of its
  right-hand columns to fit.
- `workSurface.browseListMinHeight` grows to 300 for the second line of actions, and the height of
  the work surface with it, so the preview of the Browse tab keeps the height it had.
- No new colour, size, spacing or radius is added; the new values are derived from the existing
  tokens.
- Dropped as obsolete: the facts strip height, the width of the ghost button and its disabled
  border colour. Their sources are not a reference of P4KX, and the theme says what the widgets
  are.

## Consequences

- The tests of the smallest window assert that no control is clipped by the work surface
  horizontally, and that the buttons of the Search, Localisation and Tags tabs lie in the shown
  work surface at 640 by 400 in en-GB and de-DE.
- A window lower than about 700 points scrolls the work surface vertically where it did not
  before.
