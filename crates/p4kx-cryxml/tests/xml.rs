//! Writing `CryXml` as UTF-8 XML and passing text XML through.

use std::error::Error;

use p4kx_cryxml::{Xml, read};
use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement};

type TestResult = Result<(), Box<dyn Error>>;

fn to_xml(bytes: &[u8]) -> Result<String, Box<dyn Error>> {
    let mut out = Vec::new();
    read(bytes)?.write_to(&mut out)?;
    Ok(String::from_utf8(out)?)
}

fn little(element: &TestElement) -> Vec<u8> {
    element.to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
}

#[test]
fn writes_an_indented_document_with_lf_line_endings() -> TestResult {
    let bytes = little(
        &TestElement::new("Vehicle")
            .attribute("name", "AEGS_Gladius")
            .child(
                TestElement::new("Parts").child(TestElement::new("Part").attribute("name", "Hull")),
            )
            .child(TestElement::new("Description").content("Light fighter"))
            .child(TestElement::new("Empty")),
    );

    let xml = to_xml(&bytes)?;

    assert_eq!(
        xml,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <Vehicle name=\"AEGS_Gladius\">\n\
         \x20 <Parts>\n\
         \x20   <Part name=\"Hull\"/>\n\
         \x20 </Parts>\n\
         \x20 <Description>Light fighter</Description>\n\
         \x20 <Empty/>\n\
         </Vehicle>\n",
        "document"
    );
    Ok(())
}

#[test]
fn escapes_markup_and_control_characters() -> TestResult {
    let bytes = little(
        &TestElement::new("a")
            .attribute("quote", "say \"hi\" & <bye>")
            .attribute("lines", "one\r\ntwo\tthree")
            .attribute("bell", "x\u{7}y")
            .content("1 < 2 & 3 > 2\r\n"),
    );

    let xml = to_xml(&bytes)?;

    assert!(
        xml.contains(
            "<a quote=\"say &quot;hi&quot; &amp; &lt;bye&gt;\" \
             lines=\"one&#13;&#10;two&#9;three\" bell=\"x\u{FFFD}y\">\
             1 &lt; 2 &amp; 3 &gt; 2&#13;\n</a>"
        ),
        "escaped: {xml}"
    );
    assert!(!xml.contains('\r'), "no raw CR");
    Ok(())
}

#[test]
fn replaces_characters_that_cannot_appear_in_names() -> TestResult {
    let bytes = little(
        &TestElement::new("1bad name")
            .attribute("ok-name.x", "1")
            .attribute("", "2")
            .attribute("a<b", "3"),
    );

    let xml = to_xml(&bytes)?;

    assert!(
        xml.contains("<_bad_name ok-name.x=\"1\" _=\"2\" a_b=\"3\"/>"),
        "sanitised: {xml}"
    );
    Ok(())
}

#[test]
fn keeps_non_ascii_text() -> TestResult {
    let bytes = little(&TestElement::new("Größe").attribute("wert", "Ä€😀"));

    let xml = to_xml(&bytes)?;

    assert!(xml.contains("<Größe wert=\"Ä€😀\"/>"), "unicode: {xml}");
    Ok(())
}

#[test]
fn text_xml_passes_through_byte_identical() -> TestResult {
    for text in [
        &b"<a>\r\n<b x='1'/>\r\n</a>"[..],
        b"\xEF\xBB\xBF<?xml version=\"1.0\"?>\n<a/>",
        b"\xFF\xFE<\0a\0/\0>\0",
    ] {
        let xml = read(text)?;
        assert!(matches!(xml, Xml::Text(_)), "detected as text");
        let mut out = Vec::new();
        xml.write_to(&mut out)?;
        assert_eq!(out, text, "byte-identical");
    }
    Ok(())
}

#[test]
fn the_same_input_gives_the_same_bytes() -> TestResult {
    let bytes = little(
        &TestElement::new("root")
            .attribute("b", "2")
            .attribute("a", "1")
            .child(TestElement::new("child").content("text")),
    );

    assert_eq!(to_xml(&bytes)?, to_xml(&bytes)?, "deterministic");
    Ok(())
}

#[test]
fn a_deep_tree_is_written_without_recursion_and_with_bounded_indentation() -> TestResult {
    let depth = 20_000;
    let bytes = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let element = (0..depth).fold(TestElement::new("leaf"), |inner, _| {
                TestElement::new("n").child(inner)
            });
            little(&element)
        })?
        .join()
        .map_err(|_| "building the deep tree panicked")?;

    let xml = to_xml(&bytes)?;

    assert_eq!(xml.matches("</n>").count(), depth, "every level closed");
    let widest = xml.lines().map(|line| line.len() - line.trim_start().len());
    assert_eq!(widest.max(), Some(128), "indentation stops at 64 levels");
    Ok(())
}
