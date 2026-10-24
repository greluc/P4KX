# P4KX has its own logo: data brackets around an X

ADR-0074 withdrew the earlier glyph and left the window with the platform's default icon. The
owner chose a new logo (draft "B1, white and orange").

## Decision

- The logo is data brackets around an X on a 130 unit square: a `CANVAS` tile with `rx` 24, two
  `WHITE` brackets of stroke 9, and the two strokes of the X (back `ACCENT_DARK`, front `PRIMARY`)
  of stroke 11 with round caps. The exact geometry is in `docs/design/logo.md`.
- Every colour is a `p4kx_gui::theme::palette` constant; a test compares the SVG files with the
  palette and the geometry constants.
- There are two variants: the **tile** (the icon) and the **mark** (no tile, for the header and
  the README).
- `docs/design/p4kx-logo.svg` and `docs/design/p4kx-mark.svg` are the reference.
  `p4kx_gui::theme::logo` rasterises the same geometry with tiny-skia for the window icon and the
  header mark.
- The window and taskbar icon, `p4kx-gui.exe`, `p4kx.exe` (a multi-size `.ico` embedded by
  `build.rs` through `winresource`, only when the target is Windows) and the MSI
  (`ARPPRODUCTICON` and the Start-menu shortcut) show the tile.
- The header shows the mark before the wordmark; the desktop spec carries `brandMarkSize` and
  `brandGap`, and the theme tests compare them.
- The `.ico`, the 512 px PNG and the 1280 x 640 social-preview PNG are generated from the
  geometry and committed; a test fails when they differ from the geometry.

## Consequences

- ADR-0074's statement that the header shows the wordmark only and that there is no window icon
  is superseded by this ADR; its other decisions stand.
- A change to the logo is a change to the geometry constants, the two SVG files and
  `docs/design/logo.md` together, followed by regenerating the assets.
- The owner uploads the social preview in the repository settings; GitHub has no API for it.
