// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The file and movie header of GFx and SWF movies, compressed and uncompressed.

use p4kx_gfx::{Compression, GfxError, MAX_MOVIE_LEN, Movie, Signature};
use p4kx_testkit::{TestMovie, TestSignature};

fn parse(signature: TestSignature) -> Movie {
    Movie::parse(TestMovie::new(signature).frame_count(3).bytes())
        .unwrap_or_else(|error| unreachable!("invalid movie: {error}"))
}

#[test]
fn every_signature_parses_with_its_compression() {
    for (test, signature, compression) in [
        (TestSignature::Fws, Signature::Fws, Compression::None),
        (TestSignature::Cws, Signature::Cws, Compression::Zlib),
        (TestSignature::Zws, Signature::Zws, Compression::Lzma),
        (TestSignature::Gfx, Signature::Gfx, Compression::None),
        (TestSignature::Cfx, Signature::Cfx, Compression::Zlib),
    ] {
        let movie = parse(test);
        let header = movie.header();
        assert_eq!(header.signature, signature, "{signature}");
        assert_eq!(header.signature.compression(), compression, "{signature}");
        assert_eq!(header.version, 10, "{signature}");
        assert_eq!(header.frame_count, 3, "{signature}");
        assert!(
            (header.frame_rate - 30.0).abs() < f32::EPSILON,
            "{signature}"
        );
        assert_eq!(header.frame_size.width(), 1024 * 20, "{signature}");
        assert_eq!(header.frame_size.height(), 768 * 20, "{signature}");
        assert_eq!(movie.len(), header.declared_len as usize, "{signature}");
        assert_eq!(
            header.signature.is_gfx(),
            matches!(signature, Signature::Gfx | Signature::Cfx),
            "{signature}"
        );
    }
}

#[test]
fn compressed_and_uncompressed_movies_hold_the_same_tags() {
    let movie = TestMovie::new(TestSignature::Fws).tag(1, &[]);
    let plain = Movie::parse(movie.bytes()).expect("plain");
    for signature in [TestSignature::Cws, TestSignature::Zws] {
        let compressed = TestMovie::new(signature).tag(1, &[]);
        let compressed = Movie::parse(compressed.bytes()).expect("compressed");
        assert_eq!(
            compressed.assets().tag_counts,
            plain.assets().tag_counts,
            "{signature:?}"
        );
    }
}

#[test]
fn an_unknown_signature_is_an_error() {
    let mut bytes = TestMovie::new(TestSignature::Fws).bytes();
    bytes[..3].copy_from_slice(b"PNG");
    assert!(matches!(
        Movie::parse(bytes),
        Err(GfxError::Signature { found }) if found == b"PNG"
    ));
}

#[test]
fn a_file_shorter_than_its_header_is_an_error() {
    assert!(matches!(
        Movie::parse(b"GFX".to_vec()),
        Err(GfxError::Parse(_))
    ));
}

#[test]
fn a_declared_length_below_the_header_is_an_error() {
    let mut bytes = TestMovie::new(TestSignature::Cws).bytes();
    bytes[4..8].copy_from_slice(&7_u32.to_le_bytes());
    assert!(matches!(
        Movie::parse(bytes),
        Err(GfxError::DeclaredLength { declared: 7 })
    ));
}

#[test]
fn a_compressed_movie_declaring_more_than_the_limit_is_refused() {
    for signature in [TestSignature::Cws, TestSignature::Zws, TestSignature::Cfx] {
        let mut bytes = TestMovie::new(signature).bytes();
        let declared = u32::try_from(MAX_MOVIE_LEN + 1).expect("fits");
        bytes[4..8].copy_from_slice(&declared.to_le_bytes());
        assert!(
            matches!(Movie::parse(bytes), Err(GfxError::TooLarge { limit }) if limit == MAX_MOVIE_LEN),
            "{signature:?}"
        );
    }
}

#[test]
fn a_stream_decompressing_beyond_the_declared_length_is_cut_there() {
    for signature in [TestSignature::Cws, TestSignature::Zws, TestSignature::Cfx] {
        let movie = TestMovie::new(signature).tag(1, &[]).tag(1, &[]);
        let mut bytes = movie.bytes();
        let declared = u32::from_le_bytes(bytes[4..8].try_into().expect("four bytes"));
        bytes[4..8].copy_from_slice(&(declared - 4).to_le_bytes());
        let movie = Movie::parse(bytes).expect("cut movie");
        assert_eq!(
            movie.len(),
            declared as usize - 4,
            "{signature:?}: cut at the declared length"
        );
        assert_eq!(
            movie.assets().timeline.frames,
            1,
            "{signature:?}: the second frame is cut off"
        );
    }
}

#[test]
fn corrupt_compressed_data_is_an_error() {
    for signature in [TestSignature::Cws, TestSignature::Zws] {
        let mut bytes = TestMovie::new(signature).bytes();
        let middle = bytes.len() / 2 + 4;
        bytes.truncate(middle);
        bytes.extend_from_slice(&[0xFF; 4]);
        assert!(
            matches!(Movie::parse(bytes), Err(GfxError::Decompress { .. })),
            "{signature:?}"
        );
    }
}

#[test]
fn a_truncated_movie_header_is_an_error() {
    let mut bytes = TestMovie::new(TestSignature::Gfx).bytes();
    bytes.truncate(10);
    bytes[4..8].copy_from_slice(&10_u32.to_le_bytes());
    assert!(matches!(Movie::parse(bytes), Err(GfxError::Parse(_))));
}
