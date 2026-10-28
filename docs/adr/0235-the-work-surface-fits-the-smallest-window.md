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

- The room of a work tab keeps 572 by 480 points (`workSurface.minWidth`, `minHeight`). The width
  is the smallest window less the 40 point rail, the 12 point scrollbar a low window shows and the
  8 point gutter on each side of the window, derived from those constants. The height is the Browse
  list minimum plus the preview height. A window of 640 points scrolls the work surface
  vertically only.
- The sidebar collapses below 872 points, the width of the sidebar plus the smallest work surface.
- A toolbar that does not fit 572 points puts its actions in a line of their own, right-aligned,
  and that line wraps when it is still too narrow. The Entry table shrinks the initial widths of its
  right-hand columns to fit.
- The height of a second line of actions (`workSurface.wrappedActionsHeight`, 56) is reserved only
  while a toolbar really wraps its actions: then the work surface and the Browse list keep that
  much more. Otherwise `browseListMinHeight` stays 244 and `minHeight` 480, so a wide window does
  not scroll more than before.
- No new colour, size, spacing or radius is added; the new values are derived from the existing
  tokens.
- Dropped as obsolete: the facts strip height, the width of the ghost button and its disabled
  border colour. Their sources are not a reference of P4KX, and the theme says what the widgets
  are.

## Consequences

- The tests of the smallest window assert that no control is clipped by the work surface
  horizontally, and that the buttons of the Search, Localisation and Tags tabs lie in the shown
  work surface at 640 by 400 in en-GB and de-DE.
- Only a window that wraps its actions, in German at the smallest widths, scrolls the work
  surface 56 points earlier vertically.
