# The window follows the KRT design system: dark only, Lato only

The roadmap planned the KRT look with Audiowide and Lato in a light and a dark theme. The KRT
design system, the authority for colour, form and type across the KRT tools, has since dropped
Audiowide for Lato alone and has no light theme; platform implementations may deviate only where
the platform forces it, never in colour, form or type. P4K-Extractor therefore takes the design
system's colour tokens 1:1 (`p4kx_gui::theme::palette`) and its type scale with 1rem as 16
points, sets every text in Lato (Regular, and Bold for headings), keeps every corner square, and stays dark whatever the system theme. There is
no theme setting.

The native Windows frame stays the system frame, so snap layouts, shadow and rounded corners
remain; DWM tints its caption Grau 4 with Grau 1 text and a Grau 3 border in dark mode. That
needs `DwmSetWindowAttribute`, the only foreign call of the workspace. It lives in the
`cfg(windows)` module `p4kx_gui::dwm`: `p4kx-gui` repeats the workspace lints with
`unsafe_code = "deny"` instead of `"forbid"`, that module alone expects the lint, and
`cargo xtask lints` fails if any other crate leaves the workspace lints, `p4kx-gui` changes
anything else, or another source file names `unsafe_code`. Each unsafe block sits alone in a
small safe function whose rustdoc `# Safety` section justifies it, since a plain `// SAFETY:`
comment is not used; that function expects `clippy::undocumented_unsafe_blocks` with a reason. On Linux the module is not compiled and the window keeps the default frame.

## Considered Options

- Audiowide headings and a derived light theme, as first planned: matches the roadmap, but
  diverges from every other KRT tool and from the design system's rule on type and colour.
- A custom-drawn, frameless window in KRT colours: complete control, but loses snap layouts,
  the system shadow and the rounded corners of Windows 11.
- The DWM call in a separate crate with its own lint level: keeps `forbid` in `p4kx-gui`, but
  adds a crate for four calls.
