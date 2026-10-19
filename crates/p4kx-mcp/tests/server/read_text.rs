use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement, TestEntry};
use serde_json::{Value, json};

use crate::support::{Install, Server, TestResult};

fn install() -> TestResult<Install> {
    let ship = TestElement::new("Ship")
        .attribute("name", "Gladius")
        .child(TestElement::new("Part").attribute("class", "wing"))
        .child(TestElement::new("Seat").content("pilot"))
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Big);
    let mut huge = b"CryXmlB\0".to_vec();
    huge.resize((32 << 20) + 1, 0);
    let mut long = "ä".repeat(5000).into_bytes();
    long.extend_from_slice(b"end");
    Install::new(vec![
        TestEntry::zstd("Data/ship.xml", ship),
        TestEntry::zstd("Data/huge.xml", huge),
        TestEntry::stored("Data/be.txt", b"\xFE\xFF\x00h\x00\xE4\x00x".to_vec()),
        TestEntry::stored("Data/le.txt", b"\xFF\xFEh\x00\xE4\x00x\x00".to_vec()),
        TestEntry::stored("Data/odd.txt", b"\xFF\xFEh\x00\xE4".to_vec()),
        TestEntry::stored("Data/surrogate.txt", b"\xFF\xFE\x00\xD8h\x00".to_vec()),
        TestEntry::stored("Data/cut.txt", b"abc\xC3".to_vec()),
        TestEntry::stored("Data/control.txt", b"a\x01b".to_vec()),
        TestEntry::stored("Data/delete.txt", b"a\x7Fb".to_vec()),
        TestEntry::stored("Data/latin1.txt", b"gr\xFC\xDFe".to_vec()),
        TestEntry::stored("Data/feed.txt", b"a\x0Cb\tc\r\n".to_vec()),
        TestEntry::deflated("Data/long.txt", long),
    ])
}

fn whole_text(server: &mut Server, path: &str, max_bytes: u32) -> TestResult<String> {
    let mut text = String::new();
    let mut offset = Value::from(0);
    loop {
        let page = server.ok(
            "read_entry_text",
            &json!({ "path": path, "offset": offset, "max_bytes": max_bytes }),
        )?;
        assert_eq!(
            page.get("offset"),
            Some(&offset),
            "each page starts where the last ended"
        );
        text.push_str(page.get("text").and_then(Value::as_str).unwrap_or_default());
        match page.get("next_offset") {
            Some(next) => offset = next.clone(),
            None => return Ok(text),
        }
    }
}

#[test]
fn paging_a_cryxml_to_its_end_joins_back_into_the_whole_xml() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let whole = server.ok("read_entry_text", &json!({ "path": "Data/ship.xml" }))?;
    let xml = whole["text"].as_str().unwrap_or_default();
    assert_eq!(
        xml,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Ship name=\"Gladius\">\n  \
         <Part class=\"wing\"/>\n  <Seat>pilot</Seat>\n</Ship>\n",
        "the converted XML"
    );
    assert_eq!(whole["size"], xml.len(), "the size of the whole XML");
    assert_eq!(
        whole_text(&mut server, "Data/ship.xml", 7)?,
        xml,
        "joined pages"
    );
    let long = whole_text(&mut server, "Data/long.txt", 4097)?;
    assert_eq!(
        long,
        format!("{}end", "ä".repeat(5000)),
        "UTF-8 pages beyond the first 8 KiB join back"
    );
    Ok(())
}

#[test]
fn utf16_of_both_byte_orders_reads_from_any_offset() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    for (path, encoding) in [("Data/be.txt", "utf-16be"), ("Data/le.txt", "utf-16le")] {
        let all = server.ok("read_entry_text", &json!({ "path": path }))?;
        assert_eq!(
            (&all["encoding"], &all["text"], &all["size"]),
            (&json!(encoding), &json!("häx"), &json!(4)),
            "{path}"
        );
        let inside = server.ok("read_entry_text", &json!({ "path": path, "offset": 2 }))?;
        assert_eq!(
            (&inside["offset"], &inside["text"]),
            (&json!(3), &json!("x")),
            "{path}: an offset inside ä moves behind it"
        );
    }
    Ok(())
}

#[test]
fn malformed_and_control_character_text_is_binary() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    for path in [
        "Data/odd.txt",
        "Data/surrogate.txt",
        "Data/cut.txt",
        "Data/control.txt",
        "Data/delete.txt",
        "Data/latin1.txt",
    ] {
        let message = server.error("read_entry_text", &json!({ "path": path }))?;
        assert!(message.contains("binary data"), "{path}: {message}");
    }
    let feed = server.ok("read_entry_text", &json!({ "path": "Data/feed.txt" }))?;
    assert_eq!(
        feed["text"], "a\u{c}b\tc\r\n",
        "tab, line breaks and form feed are text"
    );
    Ok(())
}

#[test]
fn a_cryxml_too_large_to_decode_points_to_the_bytes_tool() -> TestResult {
    let install = install()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let message = server.error("read_entry_text", &json!({ "path": "Data/huge.xml" }))?;
    assert!(
        message.contains("too large") && message.contains("read_entry_bytes"),
        "{message}"
    );
    let bytes = server.ok(
        "read_entry_bytes",
        &json!({ "path": "Data/huge.xml", "offset": 32 << 20, "max_bytes": 8 }),
    )?;
    assert_eq!(
        bytes["data"], "AA==",
        "the last byte through the bytes tool"
    );
    Ok(())
}
