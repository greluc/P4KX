# The window follows the binding KRT design system: dark only, Lato only

The current `main` of [krt-profit/design-system](https://github.com/krt-profit/design-system) is
the binding source for every GUI of P4K-Extractor (see "GUI design" in `CLAUDE.md`). It has no
light theme and sets everything in Lato; platform implementations may deviate only where the
platform forces it, never in colour, form or type. The desktop metrics derived from it for this
mouse and keyboard tool are fixed in [docs/design/desktop-spec.md](../design/desktop-spec.md) and
its machine-readable [p4kx-desktop-spec.json](../design/p4kx-desktop-spec.json).

- `p4kx_gui::theme` mirrors the design system's `tokens.json` 1:1: `palette` and `categorical`
  every colour token, `tokens` the type scale, weights, tracking, line height, spacing, radii,
  glows and shadows, `desktop` the metrics of the desktop spec. A copy of `tokens.json` lives in
  `crates/p4kx-gui/src/theme/fixtures/` (design system `264e7ac`); unit tests compare every token
  and the desktop metrics against it and against the desktop spec. When the design system
  changes, the fixture is replaced and the theme follows until the tests pass.
- The window ships Lato Light, Regular and Bold. Text below 16 points is Regular, Light only
  prose from 16 points, Bold labels, buttons and headings. Every corner is square; the radio is
  the only round control.
- `p4kx_gui::widgets` draws the components after the contracts of `krt-components.css`: the
  action hierarchy (one filled CTA per context, outline, ghost), checkbox, radio, tab nav,
  segmented quick filters, data rows, panel-header, modal and window frames with the orange top
  edge and corner brackets, alerts, spinner, progress meter, the brand.
- The window stays dark whatever the system theme; there is no theme setting.
- The native Windows frame stays the system frame, so snap layouts, shadow and rounded corners
  remain; DWM tints its caption Grau 4 with Grau 1 text and a Grau 3 border in dark mode. That
  needs `DwmSetWindowAttribute`, the only foreign call of the workspace. It lives in the
  `cfg(windows)` module `p4kx_gui::dwm`: `p4kx-gui` repeats the workspace lints with
  `unsafe_code = "deny"` instead of `"forbid"`, that module alone expects the lint, and
  `cargo xtask lints` fails if any other crate leaves the workspace lints, `p4kx-gui` changes
  anything else, or another source file names `unsafe_code`. Each unsafe block sits alone in a
  small safe function whose rustdoc `# Safety` section justifies it; that function expects
  `clippy::undocumented_unsafe_blocks` with a reason. On Linux the module is not compiled and the
  window keeps the default frame.

## Forced deviations

Each is forced by egui 0.37 or Windows; colour, form and type stay as the design system sets them.

| Design system | Window | Forced by |
| :-- | :-- | :-- |
| `text-transform: uppercase` | Upper case is set when the text is drawn; the accessible name keeps the Fluent text | egui has no text transform |
| `font-variant-numeric: tabular-nums` | Hex bytes sit in fixed cells; other numbers use Lato's default figures | egui cannot switch OpenType features |
| `backdrop-filter: blur` behind modals | Flat scrim `rgba(0,0,0,0.8)` | egui cannot blur |
| Scrollbar thumb in a 2 point channel, hairline on the track | Thumb as wide as the 12 point bar (8 in the sidebar), no track hairline | egui's scroll bar paints thumb and track as two plain rectangles |
| Resize grip: 6 point canvas strip with a Grau 2 grip, accent-light on hover | A hairline separator that turns orange on hover and while dragging | egui draws the panel resize line itself |
| Menu link open: selected tint plus 3 point orange underline | Selected tint and white text, no underline | egui's menu bar draws its own buttons |
| Dropdown entry hover: Grau 3 and orange text; keyboard-active entry filled orange | Grau 3 and orange text on hover; keyboard focus outlined | egui's menu entries share the button visuals |
| Tooltip on `rgba(0,0,0,0.95)` with an orange `0 0 10px` glow | Tooltip on Grau 4 with the orange hairline and `shadow-dropdown` | egui gives tooltips the popup frame |
| `:focus-visible` only for keyboard focus | The focus ring shows while a control has focus, also after a click | egui does not tell keyboard focus from pointer focus |
| Logo as SVG | The glyph is rasterised from its geometry for the window icon and the header mark | egui and Windows need raster images |
| Web footer `.krt-footer` 16/32 padding | Status bar 32 points high (desktop spec) | a desktop status line, decided in the desktop spec |
| `.krt-segmented--sm` items: `fs-2xs` Bold, upper case, 0.05 em | Quick filters 12.8 points Regular (Bold when pressed), as written, no tracking | the filters are glob data (desktop spec D.6 and its quickFilter entry) |
| Touch target 44 points for data rows | 36 point rows, 44 point controls (desktop spec D.1) | mouse and keyboard tool, decided in the desktop spec |

## Considered Options

- Audiowide headings and a derived light theme, as first planned: matches the roadmap, but
  diverges from every other KRT tool and from the design system's rule on type and colour.
- egui's experimental widget theming (`StyleProvider`): would style egui's own widgets, but is
  behind an unstable feature; drawing the components keeps the contracts under test.
- A custom-drawn, frameless window in KRT colours: complete control, but loses snap layouts,
  the system shadow and the rounded corners of Windows 11.
- The DWM call in a separate crate with its own lint level: keeps `forbid` in `p4kx-gui`, but
  adds a crate for four calls.
