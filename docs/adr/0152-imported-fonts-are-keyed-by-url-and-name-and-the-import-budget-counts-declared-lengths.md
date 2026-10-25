# Imported fonts are keyed by URL and name, and the import budget counts declared lengths

Decided for issue #390, the follow-ups of the review of PR #385. Amends ADR-0048.

## Decision

- **Key by (URL, name).** `resolve_fonts` resolves every (URL, name) pair of the imports a text
  uses once, compared ignoring ASCII case, and stamps the font it takes with `Font::import_url`.
  `Fonts::new` finds an imported ID by the (URL, name) of its import first. Two imports of one name
  from different movies are two fonts, and a movie's own export of that name does not take the
  place of an imported one. An import that could not be resolved does not fall back to a font
  another import brought under the same name, and not to the movie's own export of that name; it
  stays a missing font and is listed. A font without `import_url` that a host passes by name
  (ADR-0048) is found by name as before. A text that names a font by `<font face>` or a font class
  has only the name, no URL, so when two URLs export one name it gets the first font of the list
  that carries the name; only text that uses an imported ID is exact.
- **The byte budget counts the larger of Entry size and declared length.** `MAX_IMPORT_BYTES` was
  checked against the Entry size, but a `CWS` or `ZWS` movie of a few kilobytes declares up to
  `MAX_MOVIE_LEN` (64 MiB) after decompression, so 16 movies could cost 1 GiB of work. The
  resolver reads the declared length from the file header (`Movie::declared_len`, which parses
  nothing else), adds it with checked arithmetic before `Movie::parse` allocates anything, and
  reports `OverBudget` when the sum would pass the budget. A header that is too short or has no
  valid signature is `Unreadable`, and so is a header that declares more than `MAX_MOVIE_LEN`,
  refused before anything is charged because `Movie::parse` would refuse it anyway and the budget
  stays for the movies that follow. The charge stays when the parse fails, because the work was
  announced.
- **The font language follows the texts.** The localisation fallback of the URL ranking
  (ADR-0048) is the folder of `UiMovie::font_language`: the language set with
  `UiMovie::with_font_language`, else that of the texts (`--lang` of `p4kx ui render`), else
  English. The folders of the UI still come first, so a language only changes which
  `Data\Localization` folder breaks the tie for a movie the UI folders do not hold. For a language
  other than English the English folder follows the chosen one (the same tail, then the same file
  there) before any other file of the UI or of another language folder, so a font movie the chosen
  language lacks resolves to English and not to whichever folder sorts first. There is no
  separate flag: a movie drawn with the German texts takes the German fonts for the same reason.
- **Ties sort by Entry index.** Entries whose normalised paths are equal (they differ only in
  case) sort by their index in the Archive, so the choice does not depend on the sort being stable.
- **Spaces draw nothing.** A space, a no-break space or a line break of a text field never draws
  a box, whether its font is missing, has no outline for it or lacks the space glyph. A tab is no
  space here and is drawn as any other character. A static text names glyph indexes, not
  characters, and keeps drawing boxes for a font without outlines.

## Notes

- `MAX_FRAME_PLACEMENTS` (2^20) cannot be reached by placements alone: a placement costs about
  120 bytes in `MAX_FRAME_BYTES` (64 MiB, about 560,000 placements), so the byte cap stops first. What the placement cap bounds
  are frames with empty display lists, each counting as one placement; the test uses 2^20 + 1000 of
  them and expects exactly one `TooManyFramePlacements`.

## Considered Options

- Recording the clash only in ADR-0048 and leaving the lookup by name: no movie of the LIVE
  Archive clashes today (0 of 562 used imports in 357 movies), but nothing prevents a patch from
  adding one, and the wrong glyph set would then be drawn without a report.
- Counting the bytes after the parse: the allocation is the thing to cap, and it happens inside
  `Movie::parse`.
- A `--font-lang` flag beside `--lang`: one more flag for a case (texts in one language, fonts in
  another) the game never has; `UiMovie::with_font_language` covers it for library users.
