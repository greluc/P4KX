# The theme takes over the table stripe and the inset tile

Decided for issue #566. Builds on ADR-0006 and ADR-0074.

The design tokens P4KX's theme follows moved on after the measured palette: a `table-stripe-bg`
token for the zebra rows of tables, and an inset variant of the tiles that sit in a panel. The
theme (`p4kx_gui::theme`) and `docs/design/` take over what applies to a P4KX surface, under
P4KX's own names.

## Decision

- **Zebra rows.** `palette::TABLE_STRIPE_BG` (`table-stripe-bg`) is `color-control` at 50 %,
  `rgba(55, 62, 67, 0.5)`, which composes to `#30353A` on the panel. It replaces the white at 2 %
  that landed one lightness step above the row and could not be followed across a wide table.
  `desktop::ZEBRA`, which every striped table and egui's `faint_bg_color` use, is the same
  constant. The row hover `control-bg` (`#373E43`) stays a distinct state, halfway further.
  Because the stripe is translucent it keeps its relation on a table over the inset canvas.
- **Inset tile.** A tile never takes the fill of the surface it sits on. The comparison tiles
  (`kpiTile`) sit in the work surface, so they take the bar fill `color-surface-input`
  (`#1A1D21`) instead of the panel fill; border, accent bar and chip are unchanged.
- **Token list and spec.** The token fixture holds `table-stripe-bg`, the desktop spec names the
  stripe and the tile fill by token, and the theme tests compare both.

## Contrast

- Body text `color-gray-1` on the stripe over the panel is 8.2:1, muted text `color-gray-2-text`
  5.0:1, white 12.4:1, the semantic text tints 4.9:1 to 5.9:1.
- House orange as small text on the stripe over the panel is 4.4:1, a hair under 4.5:1, where it
  is 5.0:1 on the plain panel. Orange links in striped rows keep `color-primary`; their hover
  takes `color-accent-light` (6.7:1). Switching the link colour by row parity was rejected as
  more distracting than the shortfall.
- White on the tile fill is 16.9:1.

## Left out

- Patterns P4KX does not draw (cards, matrices, tags), including the inset variants of cards:
  P4KX draws only the comparison tile.
- Every other token (palette, type, spacing, radii, glows) already equals the current values.
