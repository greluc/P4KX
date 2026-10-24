# The P4KX theme and `docs/design/` are the design reference

Until v0.3.0 the GUI followed an external design system, read fresh from its repository before
every GUI change. The look stays exactly as it is, but the external system is no longer named or
followed: P4KX carries its own reference.

## Decision

- `p4kx_gui::theme` and `docs/design/` (the desktop spec, its machine-readable form and the logo)
  are the reference for every GUI change. No external design system is read or followed.
- The tokens, class names and fixtures carry neutral names (`planet-hurston`, `item-class-*`,
  `.modal`, `.segmented`); every value is unchanged.
- A new colour, size, spacing or radius is added to the theme and to `docs/design/` together.
- A deviation from the desktop spec is allowed only where egui or the platform forces it, never
  in colour, form or type, and is recorded in an ADR (ADR-0006 lists the forced ones).
- The header shows the wordmark only; the glyph stays the window and taskbar icon.

## Consequences

- A change to the look is a change to this repository alone, reviewed against the desktop spec
  and a render.
- The token fixture is no longer replaced from an upstream copy; it changes with the theme.
