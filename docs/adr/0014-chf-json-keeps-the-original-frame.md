# The CHF JSON keeps the original zstd frame and padding

`p4kx chf to-json|from-json` has to give back a CHF file byte for byte. The body round-trips
exactly through the structured JSON, but the container does not: every one of the 1479 character
files we checked has non-zero bytes behind the zstd frame (left-over buffer contents such as
`efbeadde`), and the game's compressor (libzstd, single-segment frames) produces other bytes than
the pure-Rust `ruzstd` encoder we use. The JSON therefore carries, besides the Character, the
header flags, the original frame as hex and the bytes behind it without trailing zeros. On
writing, the frame is reused when it still decompresses to the body written from the JSON;
otherwise the body is compressed anew, and frame and padding are dropped for zeros. An unchanged JSON gives the
original file, an edited one a valid new file, and a JSON without `container` writes a fresh one.

## Considered Options

- Link libzstd and recompress with the game's settings: a C dependency, and still not exact,
  because the bytes behind the frame cannot be recomputed.
- Keep the whole file as hex beside the Character: exact, but an edit would have to be detected
  and merged anyway, and the JSON would carry the body twice.
- Drop byte-exactness for real files: contradicts M12.1 and M12.9.
