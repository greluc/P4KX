# Own GFx and SWF parser instead of the `swf` crate

UI movies in the Archive are Scaleform GFx files (`GFX`, `CFX`) and plain SWF files (`FWS`,
`CWS`, `ZWS`). The only maintained Rust SWF parser is Ruffle's `swf` crate (0.3.0, MIT OR
Apache-2.0); `swf-parser` (open-flash) is AGPL-3.0 and unmaintained since 2022. `swf` accepts the
GFx signatures, but returns the GFx tags (1000–1009: `ExporterInfo`, `DefineExternalImage2`,
`DefineCompactedFont`, …) as unknown bytes, decodes no bitmap, and reads a sprite's tags by
recursing into nested sprites without a depth limit, so a hostile movie can overflow the stack,
which aborts the process. We would parse the GFx tags, decode the bitmaps and convert its types
into our own anyway, and could not bound the nesting (ADR-0001's reasoning for the core formats
applies). `p4kx-gfx` therefore parses the movie itself, on `p4kx-foundation`, with every length
bounded by the data left and the limits `MAX_MOVIE_LEN`, `MAX_BITMAP_PIXELS`, `MAX_SHAPE_RECORDS`
and `MAX_SPRITE_DEPTH`. It decompresses with `flate2` and `lzma-rs` (MIT) and decodes embedded
JPEGs with `zune-jpeg` (MIT OR Apache-2.0 OR Zlib) and embedded PNGs with `png`; GIF bitmaps are
reported as not supported. Ruffle's handling of SWF quirks (the `FF D9 FF D8` marker pairs in JPEG
data, premultiplied colours in `DefineBitsLossless2` and `DefineBitsJPEG3`) and the StarBreaker
`gfx` crate (MIT) serve as references.

## Considered Options

- The `swf` crate: fastest start for the plain SWF tags, but the GFx tags, the bitmap decoding,
  the bound on nesting and the conversion into our typed values stay ours, and its dependencies
  (`bitstream-io`, `encoding_rs`, `enum-map`, `num-derive`, `simple_asn1`, …) come along.
- Porting StarBreaker's `gfx` crate: it reads only the tag summary and the first-frame placements,
  not shapes, bitmaps or fonts.
