// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The depth cap of a vehicle implementation at its boundary, on the 1 MiB stack of the Windows
//! main thread that `p4kx entity` runs on.

use std::collections::HashMap;
use std::thread;

use p4kx_assembly::{MAX_XML_ELEMENT_DEPTH, VehicleImplementation};
use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement};

use crate::vehicle_support::{DEFINITION, Files, TestResult};

const STACK: usize = 1 << 20;

fn on_small_stack<T: Send + 'static>(
    run: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    thread::Builder::new()
        .stack_size(STACK)
        .spawn(run)
        .map_err(|error| error.to_string())?
        .join()
        .map_err(|_| "the reader panicked or overflowed the stack".to_owned())
}

fn text_chain(levels: usize) -> String {
    format!(
        r#"<Vehicle name="TEST_Ship"><Parts>{}{}</Parts></Vehicle>"#,
        r#"<Part id="p" name="n">"#.repeat(levels - 2),
        "</Part>".repeat(levels - 2)
    )
}

fn cry_chain(levels: usize) -> TestElement {
    let mut element = TestElement::new("Part")
        .attribute("id", "p")
        .attribute("name", "n");
    for _ in 3..levels {
        element = TestElement::new("Part")
            .attribute("id", "p")
            .attribute("name", "n")
            .child(element);
    }
    TestElement::new("Vehicle")
        .attribute("name", "TEST_Ship")
        .child(TestElement::new("Parts").child(element))
}

fn read_text(levels: usize) -> Result<usize, String> {
    let text = text_chain(levels);
    let files = Files(HashMap::from([(
        DEFINITION.to_ascii_lowercase(),
        text.into_bytes(),
    )]));
    VehicleImplementation::read(&files, "TEST_Ship", DEFINITION, None)
        .map(|vehicle| vehicle.ids().count())
        .map_err(|error| error.to_string())
}

fn read_cry(levels: usize) -> Result<usize, String> {
    let bytes = cry_chain(levels).to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
    let files = Files(HashMap::from([(DEFINITION.to_ascii_lowercase(), bytes)]));
    VehicleImplementation::read(&files, "TEST_Ship", DEFINITION, None)
        .map(|vehicle| vehicle.ids().count())
        .map_err(|error| error.to_string())
}

#[test]
fn text_xml_at_the_cap_is_read_on_a_one_mebibyte_stack_and_one_deeper_is_refused() -> TestResult {
    let at_cap = on_small_stack(|| read_text(MAX_XML_ELEMENT_DEPTH))?;
    assert!(at_cap.is_ok(), "{at_cap:?}");
    let over = on_small_stack(|| read_text(MAX_XML_ELEMENT_DEPTH + 1))?;
    assert!(
        matches!(&over, Err(reason) if reason.contains("nested deeper")),
        "{over:?}"
    );
    Ok(())
}

#[test]
fn cryxml_at_the_cap_is_read_on_a_one_mebibyte_stack_and_one_deeper_is_refused() -> TestResult {
    let at_cap = on_small_stack(|| read_cry(MAX_XML_ELEMENT_DEPTH))?;
    assert!(at_cap.is_ok(), "{at_cap:?}");
    let over = on_small_stack(|| read_cry(MAX_XML_ELEMENT_DEPTH + 1))?;
    assert!(
        matches!(&over, Err(reason) if reason.contains("nested deeper")),
        "{over:?}"
    );
    Ok(())
}
