//! How the line diff reads each side: CryXml, text XML, UTF-16, binary and oversized Entries.

mod common;

use common::{TestResult, pair, rendered};
use p4kx_comparison::{
    DEFAULT_CONTEXT, DiffBody, Encoding, LineDiffError, LineKind, MAX_DIFF_LEN, MAX_DIFF_LINES,
    NotText, Signature, line_diff,
};
use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement, TestEntry};

fn vehicle(damage: &str) -> TestElement {
    TestElement::new("Vehicle")
        .attribute("name", "AEGS_Gladius")
        .child(TestElement::new("Part").attribute("damageMax", damage))
        .child(TestElement::new("Description").content("Light fighter"))
}

#[test]
fn cryxml_entries_are_converted_to_xml_before_diffing() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored(
            "Data\\vehicle.xml",
            vehicle("2500").to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little),
        )],
        vec![TestEntry::deflated(
            "Data\\vehicle.xml",
            vehicle("3000").to_cryxml(CryXmlSignature::Cry3Sdk, CryXmlByteOrder::Big),
        )],
    )?;

    let diff = line_diff(&old, &new, "Data\\vehicle.xml", DEFAULT_CONTEXT)?;

    assert_eq!(
        (diff.old_encoding(), diff.new_encoding()),
        (
            Some(Encoding::CryXml(Signature::CryXmlB)),
            Some(Encoding::CryXml(Signature::Cry3Sdk))
        ),
        "both converted"
    );
    let changed: Vec<_> = rendered(&diff)
        .into_iter()
        .flatten()
        .filter(|(kind, _)| *kind != LineKind::Context)
        .collect();
    assert_eq!(
        changed,
        [
            (
                LineKind::Removed,
                "  <Part damageMax=\"2500\"/>\n".to_owned()
            ),
            (LineKind::Added, "  <Part damageMax=\"3000\"/>\n".to_owned()),
        ],
        "only the changed attribute"
    );
    Ok(())
}

#[test]
fn text_xml_is_diffed_as_it_is() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored(
            "a.xml",
            "<a>\r\n  <b x='1'/>\r\n</a>\r\n",
        )],
        vec![TestEntry::stored(
            "a.xml",
            "<a>\r\n  <b x='2'/>\r\n</a>\r\n",
        )],
    )?;

    let diff = line_diff(&old, &new, "a.xml", DEFAULT_CONTEXT)?;

    assert_eq!(diff.old_encoding(), Some(Encoding::Text), "text");
    assert_eq!(
        rendered(&diff),
        [[
            (LineKind::Context, "<a>\r\n".to_owned()),
            (LineKind::Removed, "  <b x='1'/>\r\n".to_owned()),
            (LineKind::Added, "  <b x='2'/>\r\n".to_owned()),
            (LineKind::Context, "</a>\r\n".to_owned()),
        ]],
        "line terminators kept"
    );
    Ok(())
}

#[test]
fn utf16_text_with_a_byte_order_mark_is_decoded() -> TestResult {
    let utf16 = |text: &str| {
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        bytes
    };
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("a.ini", utf16("k=1\n"))],
        vec![TestEntry::stored("a.ini", utf16("k=2\n"))],
    )?;

    let diff = line_diff(&old, &new, "a.ini", DEFAULT_CONTEXT)?;

    assert_eq!(
        rendered(&diff),
        [[
            (LineKind::Removed, "k=1\n".to_owned()),
            (LineKind::Added, "k=2\n".to_owned()),
        ]],
        "decoded lines"
    );
    assert_eq!(diff.old_encoding(), Some(Encoding::Utf16Le), "encoding");
    Ok(())
}

#[test]
fn a_byte_order_mark_before_binary_data_is_binary() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("a.bin", b"\xFF\xFEa\0\0\0".to_vec())],
        vec![TestEntry::stored("a.bin", b"\xFF\xFE\x00\xD8x".to_vec())],
    )?;

    let diff = line_diff(&old, &new, "a.bin", DEFAULT_CONTEXT)?;

    assert_eq!(
        diff.body(),
        &DiffBody::NotText(NotText::Binary),
        "a NUL or an unpaired surrogate is not text"
    );
    Ok(())
}

#[test]
fn lines_that_differ_only_in_invalid_utf8_bytes_differ() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("a.cfg", b"name=caf\xE9\nx=1\n".to_vec())],
        vec![TestEntry::stored("a.cfg", b"name=caf\xE8\nx=1\n".to_vec())],
    )?;

    let diff = line_diff(&old, &new, "a.cfg", DEFAULT_CONTEXT)?;

    let DiffBody::Text(text) = diff.body() else {
        return Err("text expected".into());
    };
    let lines: Vec<(LineKind, Vec<u8>)> = text
        .hunks()
        .flat_map(|hunk| hunk.lines())
        .map(|line| (line.kind(), line.bytes().to_vec()))
        .collect();
    assert_eq!(
        lines,
        [
            (LineKind::Removed, b"name=caf\xE9\n".to_vec()),
            (LineKind::Added, b"name=caf\xE8\n".to_vec()),
            (LineKind::Context, b"x=1\n".to_vec()),
        ],
        "the raw bytes are compared"
    );
    assert!(diff.has_differences(), "differs");
    Ok(())
}

#[test]
fn cryxml_with_the_same_xml_still_differs_when_its_bytes_changed() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored(
            "v.xml",
            vehicle("1").to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little),
        )],
        vec![TestEntry::stored(
            "v.xml",
            vehicle("1").to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Big),
        )],
    )?;

    let diff = line_diff(&old, &new, "v.xml", DEFAULT_CONTEXT)?;

    assert!(rendered(&diff).is_empty(), "no hunks");
    assert!(diff.has_differences(), "the bytes changed");
    Ok(())
}

#[test]
fn a_side_with_more_lines_than_the_limit_is_too_large() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("a.txt", vec![b'\n'; MAX_DIFF_LINES + 1])],
        vec![TestEntry::stored("a.txt", "a\n")],
    )?;

    let diff = line_diff(&old, &new, "a.txt", DEFAULT_CONTEXT)?;

    assert_eq!(
        diff.body(),
        &DiffBody::NotText(NotText::TooLarge),
        "too many lines"
    );
    Ok(())
}

#[test]
fn data_shorter_than_its_stated_size_is_an_error() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("a.txt", "abc").with_uncompressed_size(10)],
        vec![TestEntry::stored("a.txt", "abc")],
    )?;

    let error = line_diff(&old, &new, "a.txt", DEFAULT_CONTEXT);

    assert!(
        matches!(
            error,
            Err(LineDiffError::SizeMismatch { .. } | LineDiffError::Read { .. })
        ),
        "typed error: {error:?}"
    );
    Ok(())
}

#[test]
fn a_side_with_a_nul_byte_is_binary() -> TestResult {
    let (_dir, old, new) = pair(
        vec![
            TestEntry::stored("a.dds", b"DDS \0\x01".to_vec()),
            TestEntry::stored("same.dds", b"DDS \0".to_vec()),
        ],
        vec![
            TestEntry::stored("a.dds", b"DDS \0\x02".to_vec()),
            TestEntry::stored("same.dds", b"DDS \0".to_vec()),
        ],
    )?;

    let changed = line_diff(&old, &new, "a.dds", DEFAULT_CONTEXT)?;
    let unchanged = line_diff(&old, &new, "same.dds", DEFAULT_CONTEXT)?;

    assert_eq!(
        changed.body(),
        &DiffBody::NotText(NotText::Binary),
        "binary"
    );
    assert!(changed.has_differences(), "changed binary differs");
    assert_eq!(
        unchanged.body(),
        &DiffBody::NotText(NotText::Binary),
        "binary"
    );
    assert!(!unchanged.has_differences(), "unchanged binary");
    Ok(())
}

#[test]
fn an_entry_larger_than_the_limit_is_not_read() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("big.txt", "a\n").with_uncompressed_size(MAX_DIFF_LEN + 1)],
        vec![TestEntry::stored("big.txt", "b\n")],
    )?;

    let diff = line_diff(&old, &new, "big.txt", DEFAULT_CONTEXT)?;

    assert_eq!(
        diff.body(),
        &DiffBody::NotText(NotText::TooLarge),
        "too large"
    );
    assert!(diff.has_differences(), "sizes differ");
    Ok(())
}

#[test]
fn corrupt_cryxml_is_an_error() -> TestResult {
    let bytes = vehicle("1").to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
    let cut = bytes.get(..bytes.len() / 2).ok_or("short")?.to_vec();
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("v.xml", bytes)],
        vec![TestEntry::stored("v.xml", cut)],
    )?;

    let error = line_diff(&old, &new, "v.xml", DEFAULT_CONTEXT);

    assert!(
        matches!(error, Err(LineDiffError::CryXml { ref path, .. }) if path == "v.xml"),
        "corrupt: {error:?}"
    );
    Ok(())
}

#[test]
fn cryxml_that_converts_to_more_than_the_limit_is_too_large() -> TestResult {
    let value = "v".repeat(1024 * 1024);
    let element = (0..130).fold(TestElement::new("a"), |element, index| {
        element.attribute(format!("a{index}"), value.as_str())
    });
    let bytes = element.to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
    assert!(
        bytes.len() < 2 * 1024 * 1024,
        "small input: {}",
        bytes.len()
    );
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("a.xml", bytes)],
        vec![TestEntry::stored("a.xml", "<a/>\n")],
    )?;

    let diff = line_diff(&old, &new, "a.xml", DEFAULT_CONTEXT)?;

    assert_eq!(
        diff.body(),
        &DiffBody::NotText(NotText::TooLarge),
        "the converted XML is capped"
    );
    Ok(())
}
