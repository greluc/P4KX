# Numbers in text data are read by one tolerant parser in the foundation

Decided for issue #304. Settles the open end of ADR-0128.

Game data in text form carries noise inside numbers: invisible format characters such as U+202C
(`mass="97858<U+202C>.0d"` in the Carrack, a `value` in the Gladius), the C literal suffixes `d`
and `f`, and sometimes a unit. A strict parser rejects such a value and a careless one reads it as
0. Both are wrong for a Basetool export.

## Decision

- **One parser, in `p4kx-foundation`.** `read_text_number` and `is_format_character` live in the
  lowest layer (ADR-0003), so format and service crates share them. The vehicle reader and the
  ship base data (`p4kx-assembly`) use them; the copy in `vehicle/read.rs` is gone.
- **Rule.** All characters of the general category Cf (Unicode 16) are dropped, the text is
  trimmed, and the longest numeric prefix is read: optional sign, digits, fraction, exponent. A
  value with no number at the start is `None`, never 0, and the caller lists it with its raw text
  (`unreadable`). A non-finite result is `None`.
- **Suffixes.** A rest of `d`, `f` or `kg` (ignoring case) after the number belongs to the number.
  Any other rest makes the read partial (`TextNumber::is_partial`), which the sums list as
  `partial` (ADR-0128); `1,5` is still the partial 1 (ADR-0050).
- **Cleaned values are counted.** `TextNumber::was_cleaned` says that format characters were
  dropped; the ship base data counts those (`cleaned`).
- **Attribute names** of vehicle XML are matched ignoring ASCII case (`Element::attribute`,
  ADR-0050), so `damagemax` and `damageMax` are one attribute.
- **Other readers stay.** Readers that parse numbers of other formats (starmap positions, material
  parameters) keep their strict parse until a measured file needs the tolerant one; they are not
  part of this decision.

## Consequences

- Output of the vehicle reader and the ship base data is unchanged, including `partial`,
  `unreadable` and `cleaned`.
- A new text-number reader calls `read_text_number` instead of writing its own prefix scan.
