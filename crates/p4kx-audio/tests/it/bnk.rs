//! Soundbank sections and the WEMs embedded in them, through the public API.

use p4kx_audio::{Codec, SectionId, Soundbank};
use p4kx_testkit::{TestBank, TestCodec, TestWem, test_signal};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn pcm(frames: u32) -> Vec<u8> {
    TestWem::pcm(TestCodec::WwisePcm, 1, 48_000, &test_signal(frames, 1)).bytes()
}

fn vorbis() -> Vec<u8> {
    TestWem::encoded(TestCodec::Vorbis, 2, 48_000, 96_000).bytes()
}

#[test]
fn the_header_states_version_and_bank_id() -> TestResult {
    let bank = Soundbank::parse(TestBank::new(0xD155_6D49).bytes())?;

    assert_eq!(bank.header().version, 154, "version");
    assert_eq!(bank.header().id, 0xD155_6D49, "bank ID");
    assert_eq!(bank.header().language_id, 0x1770_5D3E, "language ID");
    assert_eq!(bank.wems().count(), 0, "no embedded WEMs");
    Ok(())
}

#[test]
fn embedded_wems_are_listed_by_id_and_read_like_loose_wems() -> TestResult {
    let bytes = TestBank::new(1)
        .wem(300, vorbis())
        .wem(100, pcm(4_800))
        .bytes();
    let bank = Soundbank::parse(bytes.as_slice())?;

    let ids: Vec<u32> = bank.wems().map(|wem| wem.id()).collect();
    assert_eq!(ids, [100, 300], "ascending IDs");

    let pcm_wem = bank.wem(100).ok_or("WEM 100")?;
    assert_eq!(pcm_wem.bytes(), pcm(4_800).as_slice(), "the exact bytes");
    let parsed = pcm_wem.parse()?;
    assert_eq!(parsed.codec(), Codec::Pcm, "codec");
    assert_eq!(parsed.sample_count(), Some(4_800), "frames");

    let vorbis_wem = bank.wem(300).ok_or("WEM 300")?.parse()?;
    assert_eq!(vorbis_wem.codec(), Codec::Vorbis, "codec");
    assert_eq!(
        vorbis_wem.duration().map(|d| d.as_secs()),
        Some(2),
        "duration"
    );
    assert!(bank.wem(200).is_none(), "no WEM 200");
    Ok(())
}

#[test]
fn embedded_wems_borrow_the_bank_bytes() -> TestResult {
    let bytes = TestBank::new(1).wem(7, pcm(10)).bytes();
    let bank = Soundbank::parse(bytes.as_slice())?;
    let wem = bank.wem(7).ok_or("WEM 7")?;

    let range = bytes.as_ptr_range();
    assert!(
        range.contains(&wem.bytes().as_ptr()),
        "the WEM is a slice of the bank"
    );
    Ok(())
}

#[test]
fn every_section_is_kept_in_file_order_with_its_raw_body() -> TestResult {
    let bytes = TestBank::new(1)
        .wem(5, pcm(10))
        .section(*b"INIT", [1, 2, 3])
        .section(*b"PLAT", *b"Windows\0")
        .bytes();
    let bank = Soundbank::parse(bytes.as_slice())?;

    let ids: Vec<String> = bank.sections().map(|s| s.id.to_string()).collect();
    assert_eq!(
        ids,
        ["BKHD", "DIDX", "DATA", "INIT", "PLAT"],
        "section order"
    );
    let init = bank.section(SectionId::new(*b"INIT")).ok_or("INIT")?;
    assert_eq!(init.body, [1, 2, 3], "raw INIT body");
    assert_eq!(
        bank.section(SectionId::DIDX).map(|s| s.body.len()),
        Some(12),
        "one DIDX entry"
    );
    Ok(())
}

#[test]
fn stid_names_banks_by_id() -> TestResult {
    let bytes = TestBank::new(1)
        .name(1, "SSAM_CRUS_Spirit")
        .name(2, "Init")
        .bytes();
    let bank = Soundbank::parse(bytes.as_slice())?;

    let names: Vec<(u32, &str)> = bank.bank_names().collect();
    assert_eq!(names, [(1, "SSAM_CRUS_Spirit"), (2, "Init")], "names");
    Ok(())
}

#[test]
fn the_prefetched_start_of_a_streamed_wem_is_marked() -> TestResult {
    let full = vorbis();
    let start = full.get(..120).ok_or("start")?.to_vec();
    let bytes = TestBank::new(1).wem(1, start).wem(2, vorbis()).bytes();
    let bank = Soundbank::parse(bytes.as_slice())?;

    let prefetch = bank.wem(1).ok_or("WEM 1")?;
    assert!(prefetch.is_prefetch(), "WEM 1 is a prefetch");
    assert_eq!(
        prefetch.declared_len(),
        Some(full.len() as u64),
        "full length"
    );
    assert!(prefetch.parse().is_err(), "not a whole WEM");
    let whole = bank.wem(2).ok_or("WEM 2")?;
    assert!(!whole.is_prefetch(), "WEM 2 is whole");
    Ok(())
}

#[test]
fn an_owned_bank_hands_back_its_bytes() -> TestResult {
    let bytes = TestBank::new(9).wem(1, pcm(2)).bytes();
    let bank = Soundbank::parse(bytes.clone())?;

    assert_eq!(
        bank.wem(1).map(|w| w.bytes().len()),
        Some(pcm(2).len()),
        "WEM 1"
    );
    assert_eq!(bank.into_inner(), bytes, "same bytes");
    Ok(())
}
