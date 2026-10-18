//! Rebuilding Wwise Vorbis WEMs as Ogg Vorbis: codebook fallback, packets, granule positions
//! and corrupt input.

mod pages;

use std::io::Cursor;

use lewton::inside_ogg::OggStreamReader;
use p4kx_audio::{AudioError, Codebooks, Codec, OggReport, VorbisError, Wem};
use p4kx_testkit::{TestCodebook, TestCodec, TestVorbis, TestWem, test_signal};

const AOTUV_ONLY: u16 = 97;
const NEITHER: u16 = 7;
const MISSING: u16 = 598;

fn convert(bytes: Vec<u8>) -> Result<(Vec<u8>, OggReport), AudioError> {
    let wem = Wem::parse(bytes)?;
    let mut out = Vec::new();
    let result = wem.write_ogg(&mut out);
    if result.is_err() {
        assert!(out.is_empty(), "nothing is written on error");
    }
    result.map(|report| (out, report))
}

struct Decoded {
    channels: u8,
    sample_rate: u32,
    frames: usize,
}

fn try_decode(ogg: &[u8]) -> Result<Decoded, lewton::VorbisError> {
    let mut reader = OggStreamReader::new(Cursor::new(ogg))?;
    let mut frames = 0;
    while let Some(packet) = reader.read_dec_packet()? {
        frames += packet.first().map_or(0, Vec::len);
    }
    Ok(Decoded {
        channels: reader.ident_hdr.audio_channels,
        sample_rate: reader.ident_hdr.audio_sample_rate,
        frames,
    })
}

#[track_caller]
fn decode(ogg: &[u8]) -> Decoded {
    let decoded = try_decode(ogg);
    assert!(
        decoded.is_ok(),
        "a standard decoder reads the Ogg: {:?}",
        decoded.err()
    );
    decoded.unwrap_or(Decoded {
        channels: 0,
        sample_rate: 0,
        frames: 0,
    })
}

#[test]
fn a_vorbis_wem_with_default_codebooks_becomes_a_playable_ogg_of_the_right_length() {
    let sound = TestVorbis::new(2, 48_000, &[false; 40]).sample_count(4_900);

    let (ogg, report) = convert(sound.wem().bytes()).unwrap();

    assert_eq!(report.codebooks, Codebooks::Default, "codebooks");
    assert_eq!((report.channels, report.sample_rate), (2, 48_000), "format");
    assert_eq!(
        report.sample_count, 4_900,
        "trimmed to the header's samples"
    );
    assert_eq!(report.size, ogg.len() as u64, "size");
    let decoded = decode(&ogg);
    assert_eq!(
        (decoded.channels, decoded.sample_rate),
        (2, 48_000),
        "decoded format"
    );
    assert_eq!(decoded.frames, 4_900, "39 × 128 samples, trimmed to 4 900");
    let pages = pages::pages(&ogg).unwrap();
    assert_eq!(
        pages[0].lacing.len(),
        1,
        "the first page holds the ident header"
    );
    assert_eq!(
        pages[1].lacing.len(),
        2,
        "the second holds comment and setup"
    );
    assert!(
        pages[1].granule == 0 && pages[0].granule == 0,
        "header granules"
    );
    assert!(
        pages[0].first() && !pages[2].continued(),
        "BOS, fresh audio page"
    );
    assert_eq!(
        (pages[2].lacing.len(), pages[2].granule),
        (1, 0),
        "the first audio packet ends its page at granule 0"
    );
    assert!(pages.last().unwrap().last(), "EOS");
    assert_eq!(pages.last().unwrap().granule, 4_900, "the last granule");
}

#[test]
fn codebooks_that_fail_validation_fall_back_to_aotuv() {
    let sound = TestVorbis::new(1, 22_050, &[false, true, true, false])
        .codebook(TestCodebook::Library(AOTUV_ONLY));

    let (ogg, report) = convert(sound.wem().bytes()).unwrap();

    assert_eq!(report.codebooks, Codebooks::AoTuV, "codebooks");
    assert_eq!(
        decode(&ogg).frames,
        64 + 512 + 1024 + 512 + 64,
        "decoded samples"
    );
}

#[test]
fn inline_codebooks_are_used_when_no_library_fits() {
    let sound = TestVorbis::new(2, 44_100, &[false; 5]).codebook(TestCodebook::Inline);

    let (ogg, report) = convert(sound.wem().bytes()).unwrap();

    assert_eq!(report.codebooks, Codebooks::Inline, "codebooks");
    assert_eq!(decode(&ogg).frames, 4 * 128, "decoded samples");
}

#[test]
fn codebooks_that_fit_no_library_give_a_typed_error_naming_each_reason() {
    let sound = TestVorbis::new(2, 48_000, &[false; 4]);
    let rejected = |id| {
        convert(
            sound
                .clone()
                .codebook(TestCodebook::Library(id))
                .wem()
                .bytes(),
        )
    };

    let neither = rejected(NEITHER).err();
    let missing = rejected(MISSING).err();

    let no_lookup = VorbisError::InvalidSetup("a residue book has no value lookup");
    assert!(
        matches!(&neither, Some(AudioError::CodebooksRejected { default, aotuv, .. })
            if *default == no_lookup && *aotuv == no_lookup),
        "{neither:?}"
    );
    assert!(
        matches!(&missing, Some(AudioError::CodebooksRejected { default, .. })
            if *default == VorbisError::UnknownCodebook(598)),
        "{missing:?}"
    );
}

#[test]
fn long_blocks_get_the_window_flags_of_their_neighbours() {
    let flags = [false, true, true, false, true, false];
    let sound = TestVorbis::new(1, 48_000, &flags);

    let (ogg, report) = convert(sound.wem().bytes()).unwrap();

    let packets = pages::packets(&pages::pages(&ogg).unwrap());
    let audio: Vec<u8> = packets
        .iter()
        .skip(3)
        .map(|(packet, _)| packet[0])
        .collect();
    assert_eq!(
        audio,
        [0b0000, 0b1010, 0b0110, 0b0000, 0b0010, 0b0000],
        "type bit 0, mode bit, previous and next window flags"
    );
    let granules: Vec<u64> = packets
        .iter()
        .skip(3)
        .map(|(_, granule)| *granule)
        .collect();
    assert!(granules.is_sorted(), "{granules:?}");
    assert_eq!(report.sample_count, 576 + 1024 + 576 + 576 + 576, "samples");
    assert_eq!(decode(&ogg).frames, 3_328, "decoded samples");
}

#[test]
fn packets_that_keep_their_vorbis_header_bits_are_copied() {
    let flags = [false, true, true, false, true, false];
    let sound = TestVorbis::new(1, 48_000, &flags).native_packets();

    let (ogg, report) = convert(sound.wem().bytes()).unwrap();

    let packets = pages::packets(&pages::pages(&ogg).unwrap());
    let audio: Vec<Vec<u8>> = packets
        .into_iter()
        .skip(3)
        .map(|(packet, _)| packet)
        .collect();
    assert_eq!(audio, sound.packets(), "unchanged packets");
    assert_eq!(report.sample_count, 3_328, "samples");
    assert_eq!(decode(&ogg).frames, 3_328, "decoded samples");
}

#[test]
fn big_endian_wems_and_a_separate_vorb_chunk_are_read() {
    for sound in [
        TestVorbis::new(2, 32_000, &[true, false, true]).big_endian(),
        TestVorbis::new(2, 32_000, &[true, false, true]).vorb_chunk(),
    ] {
        let (ogg, report) = convert(sound.wem().bytes()).unwrap();

        assert_eq!(report.sample_count, 576 * 2, "samples");
        assert_eq!(decode(&ogg).sample_rate, 32_000, "sample rate");
    }
}

#[test]
fn other_codecs_give_a_typed_error() {
    let pcm = TestWem::pcm(TestCodec::WwisePcm, 1, 48_000, &test_signal(10, 1));

    let error = convert(pcm.bytes()).err();

    assert!(
        matches!(error, Some(AudioError::UnsupportedOggCodec(Codec::Pcm))),
        "{error:?}"
    );
}

#[test]
fn corrupt_headers_and_packets_give_typed_errors() {
    let sound = TestVorbis::new(2, 48_000, &[false; 4]);
    let truncated_data = {
        let mut data = sound.setup();
        data.splice(0..0, u16::try_from(data.len()).unwrap().to_le_bytes());
        data.extend([1, 0, 0, 9, 0, 1]);
        sound.wem().data(data)
    };
    let cases = [
        (sound.clone().block_bits(5, 11).wem().bytes(), "block sizes"),
        (sound.block_bits(11, 8).wem().bytes(), "block order"),
        (truncated_data.bytes(), "packet"),
        (
            TestWem::encoded(TestCodec::Vorbis, 2, 48_000, 0)
                .chunk(*b"vorb", vec![0; 0x2C])
                .bytes(),
            "header",
        ),
    ];

    for (bytes, case) in cases {
        let error = convert(bytes).err();

        assert!(
            matches!(error, Some(AudioError::InvalidVorbis(_))),
            "{case}: {error:?}"
        );
    }
}

#[test]
fn damaged_wems_never_panic() {
    let wem = TestVorbis::new(2, 48_000, &[false, true, false, true])
        .wem()
        .bytes();

    for len in 0..wem.len() {
        let _ignored = convert(wem[..len].to_vec());
    }
    for index in 0..wem.len() {
        for bit in 0..8 {
            let mut damaged = wem.clone();
            damaged[index] ^= 1 << bit;
            let _ignored = convert(damaged);
        }
    }
}
