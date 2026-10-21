//! The Ogg page writer: page layout, flags, granule positions, lacing and CRC.

use std::io;

use crate::pages::{packets, pages};
use p4kx_audio::OggWriter;

const SERIAL: u32 = 0x1234_5678;

fn written(write: impl FnOnce(&mut OggWriter<&mut Vec<u8>>) -> io::Result<()>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut writer = OggWriter::new(&mut out, SERIAL);
    let size = write(&mut writer).and_then(|()| writer.finish());
    assert_eq!(
        size.ok(),
        Some(out.len() as u64),
        "finish reports the bytes written"
    );
    out
}

#[test]
fn packets_come_back_in_order_with_the_granule_of_their_page() {
    let out = written(|writer| {
        writer.write_packet(b"first", 0)?;
        writer.end_page()?;
        writer.write_packet(b"second", 0)?;
        writer.write_packet(b"third", 0)?;
        writer.end_page()?;
        writer.write_packet(&[7; 300], 128)?;
        writer.write_packet(b"", 128)?;
        writer.write_packet(b"last", 1000)
    });

    let pages = pages(&out).unwrap();

    assert_eq!(pages.len(), 3, "one page per end_page and a final one");
    assert!(
        pages[0].first() && !pages[1].first(),
        "only the first page is BOS"
    );
    assert!(
        pages[2].last() && !pages[1].last(),
        "only the last page is EOS"
    );
    assert!(
        pages.iter().all(|page| !page.continued()),
        "no continued page"
    );
    assert_eq!(
        pages.iter().map(|page| page.sequence).collect::<Vec<_>>(),
        [0, 1, 2],
        "sequence numbers"
    );
    assert!(pages.iter().all(|page| page.serial == SERIAL), "serial");
    assert_eq!(
        pages[2].granule, 1000,
        "granule of the last packet on the page"
    );
    assert_eq!(
        pages[2].lacing,
        [255, 45, 0, 4],
        "lacing of 300, 0 and 4 bytes"
    );
    let packets: Vec<Vec<u8>> = packets(&pages).into_iter().map(|(p, _)| p).collect();
    assert_eq!(
        packets,
        [
            b"first".to_vec(),
            b"second".to_vec(),
            b"third".to_vec(),
            vec![7; 300],
            Vec::new(),
            b"last".to_vec()
        ],
        "packets"
    );
}

#[test]
fn a_packet_larger_than_a_page_continues_on_the_next_pages() {
    let big: Vec<u8> = (0..255 * 255 * 2 + 10_u32)
        .map(|i| (i % 251).to_le_bytes()[0])
        .collect();
    let out = written(|writer| {
        writer.write_packet(b"head", 0)?;
        writer.end_page()?;
        writer.write_packet(&big, 4096)?;
        writer.end_page()?;
        writer.write_packet(&[1; 255], 8192)
    });

    let pages = pages(&out).unwrap();

    assert!(pages.len() >= 4, "the big packet spans pages");
    assert!(
        pages.iter().all(|page| page.lacing.len() <= 255),
        "segments"
    );
    assert!(pages[2].continued(), "the second part is continued");
    assert_eq!(
        pages[1].granule,
        u64::MAX,
        "no packet ends on the first part"
    );
    let ends: Vec<(Vec<u8>, u64)> = packets(&pages);
    assert_eq!(ends.len(), 3, "three packets");
    assert!(ends[1] == (big, 4096), "the big packet is whole");
    assert_eq!(
        ends[2],
        (vec![1; 255], 8192),
        "255 bytes end with a 0 lacing value"
    );
}

#[test]
fn many_small_packets_fill_pages_of_about_four_kilobytes() {
    let out = written(|writer| {
        for index in 0..400_u64 {
            writer.write_packet(&[3; 100], index * 64)?;
        }
        Ok(())
    });

    let pages = pages(&out).unwrap();

    assert!(pages.len() > 5, "several pages: {}", pages.len());
    assert!(
        pages.iter().all(|page| page.body.len() <= 4096 + 100),
        "pages stay near the target size"
    );
    let granules: Vec<u64> = pages.iter().map(|page| page.granule).collect();
    assert!(granules.is_sorted(), "granules grow: {granules:?}");
    assert_eq!(packets(&pages).len(), 400, "every packet");
}

#[test]
fn finishing_after_an_ended_page_writes_an_empty_last_page() {
    let out = written(|writer| {
        writer.write_packet(b"only", 42)?;
        writer.end_page()
    });

    let pages = pages(&out).unwrap();

    assert_eq!(pages.len(), 2, "the ended page and an EOS page");
    assert!(
        pages[1].last() && pages[1].lacing.is_empty(),
        "empty EOS page"
    );
    assert_eq!(pages[1].granule, 42, "it keeps the last granule");
}
