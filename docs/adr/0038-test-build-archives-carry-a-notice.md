# Archives of test Channels carry a notice in every front-end

Decided for issue #348, from the legal review of 2026-10-20.

Material from the community tests (`PTU`, `EPTU`, `TECH-PREVIEW`) is confidential under the Star
Citizen Terms of Service, and `HOTFIX` builds are treated the same way. The owner decided that such
Archives may still be opened, browsed, extracted and compared, and that the tool says so every time
instead of refusing.

- The kind of an Archive is told by the name of the folder that holds it, ignoring case: `LIVE`,
  `PTU`, `EPTU`, `HOTFIX`, `TECH-PREVIEW`, anything else is unknown. This is the Channel name the
  detection of ADR-0005 already reports, so an Archive opened by path is treated like a detected one.
  `p4kx_channels::ChannelKind` holds the mapping and the English text of the notice.
- Every kind but `LIVE` and unknown gets the notice. An unknown folder name says nothing about the
  build, so it gets none.
- GUI: a warning alert of the KRT design system below the header, shown while such an Archive is
  open and in every work tab, never a modal. Its text is in the Fluent files for `de-DE` and `en-GB`.
- CLI: one line `notice: …` on stderr the first time a run opens such an Archive. Stdout, and so the
  JSON output, is never touched.
- MCP: every tool that reads an Archive of such a Channel appends the notice as one more text content
  block. The structured content and the output schemas stay as they are. The note is carried from
  `State::archive` to the result in a thread-local, because the closures of the tools run on one
  blocking thread from opening the Archive to returning the value, and this keeps the signature of
  all tools unchanged.
