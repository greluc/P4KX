// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Mutation self-test of the tree comparison: the P4KX side is read from `CryXml`, the
//! reference side from XML text, as in the real comparison.

#![expect(
    clippy::expect_used,
    reason = "the helpers parse synthetic documents; a failure fails the test"
)]

use p4kx_extraction::cryxml;
use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement};

use super::tally::Compared;
use super::xml_tree::{UNFORGE_BUGGED, UNFORGE_MOJIBAKE, XmlNode, compare};

fn element() -> TestElement {
    TestElement::new("Root")
        .attribute("name", "caf\u{e9}")
        .attribute("speed", "1.5")
        .content("body")
        .child(TestElement::new("A").attribute("id", "1"))
        .child(
            TestElement::new("B")
                .content("text")
                .child(TestElement::new("C")),
        )
}

fn ours() -> XmlNode {
    let bytes = element().to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
    let document = cryxml::parse(&bytes).expect("synthetic CryXml");
    XmlNode::from_cryxml(document.root()).expect("shallow")
}

fn theirs(xml: &str) -> XmlNode {
    let parsed = roxmltree::Document::parse(xml).expect("XML");
    XmlNode::from_roxml(parsed.root_element()).expect("shallow")
}

const REFERENCE: &str =
    "<Root name=\"caf\u{e9}\" speed=\"1.5\">body<A id=\"1\"/><B>text<C/></B></Root>";

fn detected(xml: &str) -> bool {
    matches!(compare(&ours(), &theirs(xml)), Compared::Different(_))
}

#[test]
fn the_unchanged_reference_is_equal() {
    assert_eq!(compare(&ours(), &theirs(REFERENCE)), Compared::Equal);
}

#[test]
fn every_kind_of_change_to_the_reference_tree_is_detected() {
    let mutations = [
        (
            "tag renamed",
            REFERENCE.replace("<A ", "<Z ").replace("</A>", "</Z>"),
        ),
        ("attribute value changed", REFERENCE.replace("1.5", "1.6")),
        ("attribute name changed", REFERENCE.replace("speed", "rate")),
        ("attribute deleted", REFERENCE.replace(" speed=\"1.5\"", "")),
        (
            "attribute invented",
            REFERENCE.replace("<C/>", "<C x=\"1\"/>"),
        ),
        (
            "attributes swapped",
            REFERENCE.replace(
                "name=\"caf\u{e9}\" speed=\"1.5\"",
                "speed=\"1.5\" name=\"caf\u{e9}\"",
            ),
        ),
        ("text changed", REFERENCE.replace(">text<", ">txet<")),
        ("text deleted", REFERENCE.replace("body", "")),
        ("text invented", REFERENCE.replace("<C/>", "<C>x</C>")),
        ("child deleted", REFERENCE.replace("<A id=\"1\"/>", "")),
        ("child invented", REFERENCE.replace("<C/>", "<C/><C/>")),
        (
            "children swapped",
            REFERENCE.replace(
                "<A id=\"1\"/><B>text<C/></B>",
                "<B>text<C/></B><A id=\"1\"/>",
            ),
        ),
    ];
    for (name, xml) in mutations {
        assert!(detected(&xml), "{name} is not detected");
    }
}

#[test]
fn whitespace_around_text_is_no_difference() {
    let spaced = REFERENCE
        .replace("body", "\n  body\n")
        .replace("text", " text ");
    assert_eq!(compare(&ours(), &theirs(&spaced)), Compared::Equal);
}

#[test]
fn the_known_unforge_differences_are_allowed_with_their_reasons() {
    let bugged = REFERENCE.replace("\"1.5\"", "\"BUGGED\"");
    assert_eq!(
        compare(&ours(), &theirs(&bugged)),
        Compared::Allowed(UNFORGE_BUGGED)
    );
    let mojibake: String = "caf\u{e9}".bytes().map(char::from).collect();
    let garbled = REFERENCE.replace("caf\u{e9}", &mojibake);
    assert_eq!(
        compare(&ours(), &theirs(&garbled)),
        Compared::Allowed(UNFORGE_MOJIBAKE)
    );
    let both = garbled.replace("<B>text", "<B>other");
    assert!(detected(&both), "an allowed difference hides no other");
}
