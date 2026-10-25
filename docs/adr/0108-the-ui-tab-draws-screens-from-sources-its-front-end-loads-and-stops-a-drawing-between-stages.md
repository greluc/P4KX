# The UI tab draws screens from sources its front-end loads and stops a drawing between stages

Decided for issue #173 (M14.8, the UI tab of the GUI). Builds on ADR-0003 (the layers), ADR-0101
(screen texts), ADR-0105 (screen images) and ADR-0092 (the GUI keeps the Archive open and
watches it). D.51 of the desktop spec.

## Decision

**`p4kx-screens` never opens the Archive.** `Still::render` takes the `TextSources` and the
`ImageSources` of the composer as parameters, each optional: a widget that needs a source that is
missing is listed in the report. The fonts and the `global.ini` of a language are read by
`ScreenTexts::load` in `p4kx-extraction`, the textures and vector graphics by `ArchiveImages`
there, and the front-ends (`p4kx ui render` and the GUI) wire them to the sources. `p4kx-screens`
depends on `p4kx-extraction` only in its tests. Writing the bytes of a `Still` is
`write_bytes_replacing` of `p4kx-extraction`.

**A drawing stops between its stages.** `Still::render_while` asks a `stopped` function before the
composition, after it and before the rasterisation and returns `ScreenError::Stopped`. The tab
gives it the token of its job. A stage itself is not interrupted, so a stop takes as long as the
stage that runs.

**One drawing at a time, the latest wins.** The tab keeps at most one drawing in flight and holds
only the key it wants next. A new key while a drawing runs stops that drawing; when its result
arrives it is dropped, and the key held last is drawn. `archive_changed` drops the job, which
stops it, and the export with it, so nothing of the old Archive is shown or written after the
change.

**The choices are remembered.** Size, background and format are the `[ui]` table of the settings
file (`UiRenderSettings`, `#[serde(default)]`). A value this version does not know reads as the
default, so a settings file of a later version still loads.

**The export refuses an installation.** A file inside a Star Citizen installation is refused with
the message of the other exports, before anything is drawn.

## Consequences

- The CLI and the GUI draw a screen with the same `Still`, and neither needs the other.
- A warning of the texts (the font movie or the `global.ini` could not be read or parsed) is
  produced by `ScreenTexts` and shown by the front-end; the `warning: texts are drawn without it:`
  line of the CLI therefore also covers a `global.ini` that cannot be parsed.
- Holding one key means a quick series of choices draws only the last picture, at the cost of one
  stop-and-restart after the first.

## Considered Options

- A thread per key: it spends the CPU and the Archive reads on pictures nobody sees.
- Interrupting the rasteriser itself: it would put a callback into the hot loop for a gain of a
  fraction of a second.
- Keeping the loading in `p4kx-screens`: it breaks the rule that a service crate does not open the
  Archive and made the crate depend on its sibling.
