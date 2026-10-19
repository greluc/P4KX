# The CHF JSON keeps the original zstd frame and padding

`p4kx chf to-json|from-json` has to give back a CHF file byte for byte. The body round-trips
exactly through the structured JSON, but the container does not: every one of the 1479 character
files we checked has non-zero bytes behind the zstd frame (left-over buffer contents such as
`efbeadde`), and the game's compressor (libzstd, single-segment frames) produces other bytes than
the pure-Rust `ruzstd` encoder we use. The JSON therefore carries, besides the Character, the
header flags, the original frame as hex and the bytes behind it without trailing zeros. On
writing, the frame is reused when it still decompresses to the body written from the JSON;
otherwise the body is compressed anew, and frame and padding are dropped for zeros, except for the
last 8 bytes of the file. An unchanged JSON gives the original file, an edited one a valid new
file, and a JSON without `container` writes a fresh one.

## The last 8 bytes are kept when the body changes

Game-saved files carry non-zero bytes in the last 8 bytes of the 4096; the 41 CHF files of the
LIVE Archive all hold `efbeadde efbeadde` there. The game accepts an all-zero tail as well, but a
changed body keeps the original 8 bytes: they are read from `container.frame` and
`container.padding` (the bytes of the file from the frame on, trailing zeros trimmed), so the JSON
schema does not change. The new frame is followed by zeros up to them, and the CRC-32C is
computed over the result. When the original frame itself reaches into the last 8 bytes, there is
no tail to keep and zeros are written; a JSON without `container` writes zeros. When the new
frame would reach into non-zero tail bytes (it is longer than 4072 bytes), writing fails with
`ChfError::Unwritable` instead of overwriting them; the frame limit of 4080 bytes is otherwise
unchanged.

## Considered Options

- Link libzstd and recompress with the game's settings: a C dependency, and still not exact,
  because the bytes behind the frame cannot be recomputed.
- Keep the whole file as hex beside the Character: exact, but an edit would have to be detected
  and merged anyway, and the JSON would carry the body twice.
- Drop byte-exactness for real files: contradicts M12.1 and M12.9.
