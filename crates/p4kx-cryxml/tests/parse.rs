//! Detecting and parsing `CryXml` into an element tree.

use std::error::Error;

use p4kx_cryxml::{ByteOrder, Element, Format, Signature, detect, parse};
use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement};

type TestResult = Result<(), Box<dyn Error>>;

const SIGNATURES: [(CryXmlSignature, Signature); 3] = [
    (CryXmlSignature::CryXmlB, Signature::CryXmlB),
    (CryXmlSignature::CryXml, Signature::CryXml),
    (CryXmlSignature::Cry3Sdk, Signature::Cry3Sdk),
];

const BYTE_ORDERS: [(CryXmlByteOrder, ByteOrder); 2] = [
    (CryXmlByteOrder::Little, ByteOrder::Little),
    (CryXmlByteOrder::Big, ByteOrder::Big),
];

fn sample() -> TestElement {
    TestElement::new("Vehicle")
        .attribute("name", "AEGS_Gladius")
        .attribute("size", "3")
        .child(
            TestElement::new("Parts")
                .child(TestElement::new("Part").attribute("name", "Hull"))
                .child(TestElement::new("Part").attribute("name", "Wing_Left")),
        )
        .child(TestElement::new("Description").content("Light fighter"))
}

fn names<'e>(children: impl Iterator<Item = Element<'e, 'e>>) -> Vec<&'e str> {
    children.map(Element::name).collect()
}

#[test]
fn every_signature_and_byte_order_parses_into_the_same_tree() -> TestResult {
    for (test_signature, signature) in SIGNATURES {
        for (test_order, byte_order) in BYTE_ORDERS {
            let bytes = sample().to_cryxml(test_signature, test_order);

            let document = parse(&bytes)?;

            assert_eq!(document.signature(), signature, "signature");
            assert_eq!(document.byte_order(), byte_order, "byte order");
            let root = document.root();
            assert_eq!(root.name(), "Vehicle", "{signature:?} {byte_order:?}");
            assert_eq!(root.attribute("name"), Some("AEGS_Gladius"), "attribute");
            assert_eq!(root.attribute("size"), Some("3"), "second attribute");
            assert_eq!(names(root.children()), ["Parts", "Description"], "children");
            let parts = root.children().next().ok_or("no Parts")?;
            let part_names: Vec<_> = parts
                .children()
                .filter_map(|part| part.attribute("name"))
                .collect();
            assert_eq!(part_names, ["Hull", "Wing_Left"], "grandchildren in order");
            let description = root.children().nth(1).ok_or("no Description")?;
            assert_eq!(description.content(), Some("Light fighter"), "content");
            assert_eq!(parts.content(), None, "no content");
        }
    }
    Ok(())
}

#[test]
fn attributes_keep_their_order() -> TestResult {
    let bytes = TestElement::new("a")
        .attribute("z", "1")
        .attribute("b", "2")
        .attribute("m", "3")
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);

    let document = parse(&bytes)?;

    let attributes: Vec<_> = document
        .root()
        .attributes()
        .iter()
        .map(|attribute| (attribute.name(), attribute.value()))
        .collect();
    assert_eq!(attributes, [("z", "1"), ("b", "2"), ("m", "3")], "order");
    Ok(())
}

#[test]
fn detect_names_the_signature_or_text_xml() {
    for (test_signature, signature) in SIGNATURES {
        let bytes = sample().to_cryxml(test_signature, CryXmlByteOrder::Little);
        assert_eq!(
            detect(&bytes),
            Some(Format::CryXml(signature)),
            "{signature:?}"
        );
    }
    for text in [
        &b"<a/>"[..],
        b"<?xml version=\"1.0\"?><a/>",
        b"\xEF\xBB\xBF<a/>",
        b"\r\n  \t<a/>",
        b"\xFF\xFE<\0a\0/\0>\0",
        b"\xFE\xFF\0<\0a\0/\0>",
    ] {
        assert_eq!(detect(text), Some(Format::Text), "{text:?}");
    }
    for other in [&b""[..], b"CryXm", b"hello", b"\xEF\xBB\xBFhello", b"   "] {
        assert_eq!(detect(other), None, "{other:?}");
    }
}

#[test]
fn a_wide_tree_parses() -> TestResult {
    let root = (0..2000).fold(TestElement::new("root"), |root, index| {
        root.child(TestElement::new("item").attribute("index", index.to_string()))
    });
    let bytes = root.to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Big);

    let document = parse(&bytes)?;

    assert_eq!(document.root().children().count(), 2000, "every child");
    assert_eq!(
        document
            .root()
            .children()
            .next_back()
            .and_then(|item| item.attribute("index")),
        Some("1999"),
        "last child"
    );
    Ok(())
}
