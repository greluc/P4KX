//! Hostile and malformed vehicle implementations: depth, budgets, path confinement, numbers and
//! the issues that make the patch rule of ADR-0050 visible.

#[expect(dead_code, reason = "this file uses part of the fixtures")]
mod vehicle_support;

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Mutex;

use p4kx_assembly::{LoadoutError, LoadoutFileError, LoadoutFiles, VehicleImplementation};
use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement};
use vehicle_support::{DEFINITION, Files, TestResult, files};

const DIRECTORY: &str = "Scripts/Entities/Vehicles/Implementations/Xml";

fn repeated(count: usize, template: &str) -> String {
    (0..count).fold(String::new(), |mut out, index| {
        out.push_str(&template.replace("@@", &index.to_string()));
        out
    })
}

fn patched_base(modification: &str, parts: &str) -> String {
    format!(
        r#"<Vehicle name="TEST_Ship"><Parts>{parts}</Parts><Modifications>{modification}</Modifications></Vehicle>"#
    )
}

fn patch_path(name: &str) -> String {
    format!("{DIRECTORY}/Modifications/{name}.xml")
}

fn reasons(vehicle: &VehicleImplementation) -> Vec<String> {
    vehicle.issues().iter().map(ToString::to_string).collect()
}

fn expect_issue(vehicle: &VehicleImplementation, text: &str) {
    assert!(
        vehicle
            .issues()
            .iter()
            .all(|issue| issue.kind() == "vehicle_implementation")
            && reasons(vehicle).iter().any(|reason| reason.contains(text)),
        "wanted an issue with {text:?}: {:?}",
        reasons(vehicle)
    );
}

fn read(files: &Files, modification: &str) -> Result<VehicleImplementation, LoadoutError> {
    VehicleImplementation::read(files, "TEST_Ship", DEFINITION, Some(modification))
}

#[test]
fn comments_that_hide_closing_tags_do_not_pass_the_depth_scan() -> TestResult {
    for opener in ["<!-->", "<!--->", "<!---->"] {
        let mut text = String::from("<Vehicle>");
        for _ in 0..4 {
            text.push_str(&"<a>".repeat(128));
            write!(text, "{opener} {} -->", "</a>".repeat(128))?;
        }
        let files = files(&[(DEFINITION, text.as_str())]);
        let result = VehicleImplementation::read(&files, "TEST_Ship", DEFINITION, None);
        assert!(
            matches!(&result, Err(error) if error.to_string().contains("nested deeper")),
            "{opener}: {result:?}"
        );
    }
    Ok(())
}

#[test]
fn cdata_and_processing_instructions_that_hide_tags_are_scanned_like_the_parser_does() {
    let deep = "<a>".repeat(200);
    for hidden in [
        format!("<![CDATA[]]> <?p </a> ?>{deep}"),
        format!("<?p?>{deep}"),
        format!("<![CDATA[</a>]]>{deep}"),
    ] {
        let text = format!("<Vehicle>{hidden}</Vehicle>");
        let files = files(&[(DEFINITION, text.as_str())]);
        let result = VehicleImplementation::read(&files, "TEST_Ship", DEFINITION, None);
        assert!(result.is_err(), "{hidden:.40}");
    }
}

#[test]
fn cryxml_nodes_that_share_a_huge_attribute_range_are_refused() -> TestResult {
    let mut root = TestElement::new("Vehicle");
    for index in 0..60_000 {
        root = root.attribute(format!("a{index}"), format!("v{index}"));
    }
    for _ in 0..100 {
        root = root.child(TestElement::new("Parts"));
    }
    let mut bytes = root.to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
    let nodes = 8 + 9 * 4;
    for child in 1..=100 {
        let at = nodes + child * 28;
        bytes
            .get_mut(at + 8..at + 10)
            .ok_or("no node")?
            .copy_from_slice(&60_000_u16.to_le_bytes());
        bytes
            .get_mut(at + 16..at + 20)
            .ok_or("no node")?
            .copy_from_slice(&0_u32.to_le_bytes());
    }
    let files = Files(HashMap::from([(DEFINITION.to_ascii_lowercase(), bytes)]));
    let result = VehicleImplementation::read(&files, "TEST_Ship", DEFINITION, None);
    assert!(
        matches!(&result, Err(error) if error.to_string().contains("more attributes")
            || error.to_string().contains("more bytes")),
        "{:?}",
        result.as_ref().map(|_| ())
    );
    Ok(())
}

#[test]
fn a_patch_copied_into_many_elements_with_one_id_is_refused_and_the_base_stays() -> TestResult {
    let parts = repeated(2000, r#"<Part id="x" name="p@@"/>"#);
    let base = patched_base(
        r#"<Modification name="M" patchFile="Modifications/P"/>"#,
        &parts,
    );
    let children = repeated(50, r#"<Part name="child_@@" class="ItemPort"/>"#);
    let patch = format!(r#"<Modifications><Part id="x">{children}</Part></Modifications>"#);
    let files = files(&[
        (DEFINITION, base.as_str()),
        (patch_path("P").as_str(), patch.as_str()),
    ]);
    let vehicle = read(&files, "M")?;
    expect_issue(&vehicle, "than are allowed");
    assert_eq!(vehicle.patch_path(), None);
    assert_eq!(vehicle.parts().len(), 2000);
    assert!(
        vehicle
            .parts()
            .iter()
            .all(|part| part.children().is_empty())
    );
    Ok(())
}

#[test]
fn many_elems_for_one_shared_id_are_refused_and_the_base_stays() -> TestResult {
    let parts = repeated(2000, r#"<Part id="x" name="p@@"/>"#);
    let elems = repeated(300, r#"<Elem idRef="x" name="a@@" value="v"/>"#);
    let base = patched_base(
        &format!(r#"<Modification name="M"><Elems>{elems}</Elems></Modification>"#),
        &parts,
    );
    let files = files(&[(DEFINITION, base.as_str())]);
    let vehicle = read(&files, "M")?;
    expect_issue(&vehicle, "than are allowed");
    assert!(
        vehicle
            .parts()
            .iter()
            .all(|part| part.attribute("a0").is_none())
    );
    Ok(())
}

#[test]
fn a_patch_that_nests_the_tree_too_deep_is_refused_and_the_base_stays() -> TestResult {
    let chain = |depth: usize, inner: &str| {
        format!(
            "{}{inner}{}",
            r#"<Part name="n">"#.repeat(depth),
            "</Part>".repeat(depth)
        )
    };
    let base = patched_base(
        r#"<Modification name="M" patchFile="Modifications/P"/>"#,
        &chain(100, r#"<Part id="x" name="leaf"/>"#),
    );
    let patch = format!(
        r#"<Modifications><Part id="x">{}</Part></Modifications>"#,
        chain(100, "")
    );
    let files = files(&[
        (DEFINITION, base.as_str()),
        (patch_path("P").as_str(), patch.as_str()),
    ]);
    let vehicle = read(&files, "M")?;
    expect_issue(&vehicle, "nested deeper");
    assert_eq!(vehicle.patch_path(), None);
    assert_eq!(vehicle.iter().count(), 101);
    Ok(())
}

#[test]
fn a_dangling_patch_file_is_an_issue_even_when_a_fallback_is_applied() -> TestResult {
    let base = patched_base(
        r#"<Modification name="M" patchFile="Modifications/Nope"/>"#,
        r#"<Part name="body"/>"#,
    );
    let patch = r#"<Modifications><Parts><Part name="other"/></Parts></Modifications>"#;
    let files = files(&[
        (DEFINITION, base.as_str()),
        (patch_path("TEST_Ship_M").as_str(), patch),
    ]);
    let vehicle = read(&files, "M")?;
    expect_issue(&vehicle, "Nope");
    assert!(
        vehicle
            .patch_path()
            .is_some_and(|path| path.ends_with("TEST_Ship_M.xml"))
    );
    assert_eq!(
        vehicle
            .parts()
            .first()
            .map(p4kx_assembly::VehiclePart::name),
        Some("other")
    );
    Ok(())
}

#[test]
fn patch_children_and_ids_without_a_counterpart_are_issues_and_in_the_report() -> TestResult {
    let base = patched_base(
        r#"<Modification name="M" patchFile="Modifications/P"><Elems><Elem idRef="gone" name="a" value="b"/></Elems></Modification>"#,
        r#"<Part id="x" name="body"/>"#,
    );
    let patch = r#"<Modifications>
        <Part id="x" name="renamed"/>
        <Part id="nothing" name="n"/>
        <MovementParams speed="1"/>
        <Parts><Part name="other"/></Parts>
    </Modifications>"#;
    let files = files(&[
        (DEFINITION, base.as_str()),
        (patch_path("P").as_str(), patch),
    ]);
    let vehicle = read(&files, "M")?;
    expect_issue(&vehicle, "MovementParams");
    expect_issue(&vehicle, "nothing");
    let report = vehicle.modification_report().ok_or("no report")?;
    assert_eq!(report.wholesale_children(), 2);
    assert_eq!(report.vehicle_children(), 0);
    assert_eq!(report.id_children(), 2);
    assert_eq!(report.unmatched_children(), ["MovementParams"]);
    assert_eq!(report.unmatched_ids(), ["nothing"]);
    assert_eq!(report.unmatched_elems(), ["gone"]);
    Ok(())
}

struct Recording {
    inner: Files,
    log: Mutex<Vec<String>>,
}

impl LoadoutFiles for Recording {
    fn read(&self, path: &str) -> Result<Vec<u8>, LoadoutFileError> {
        if let Ok(mut log) = self.log.lock() {
            log.push(path.to_owned());
        }
        self.inner.read(path)
    }
}

fn confined(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    let lower = lower.strip_prefix("data/").unwrap_or(&lower);
    lower.starts_with(&DIRECTORY.to_ascii_lowercase()) && lower.split('/').all(|part| part != "..")
}

#[test]
fn no_path_outside_the_implementation_folder_is_ever_read() -> TestResult {
    let base = patched_base(
        r#"<Modification name="../../Evil" patchFile="..\..\Evil"/><Modification name="M" patchFile="Scripts/Other/x.xml"/><Modification name="a/b" patchFile="C:/x.xml"/>"#,
        r#"<Part name="body"/>"#,
    );
    let cases = [
        ("TEST_Ship", format!("{DIRECTORY}/../../../Secret.xml"), "M"),
        (
            "TEST_Ship",
            "Scripts/Other/Foo.xml".to_owned(),
            "../../Evil",
        ),
        ("TEST_Ship", DEFINITION.to_owned(), "a/b"),
        ("../Evil", "../Evil.xml".to_owned(), "M"),
        ("a/b", DEFINITION.to_owned(), "M"),
        ("TEST_Ship", "C:/Windows/x.xml".to_owned(), ""),
    ];
    for (class_name, definition, modification) in cases {
        let recording = Recording {
            inner: files(&[(DEFINITION, base.as_str())]),
            log: Mutex::new(Vec::new()),
        };
        let modification = (!modification.is_empty()).then_some(modification);
        let _ = VehicleImplementation::read(&recording, class_name, &definition, modification);
        let log = recording.log.lock().map_err(|_| "poisoned")?.clone();
        assert!(
            log.iter().all(|path| confined(path)),
            "{class_name} {definition} {modification:?}: {log:?}"
        );
    }
    Ok(())
}

#[test]
fn a_definition_outside_the_folder_is_not_read_but_the_class_file_is() -> TestResult {
    let recording = Recording {
        inner: files(&[(
            DEFINITION,
            patched_base("", r#"<Part name="body"/>"#).as_str(),
        )]),
        log: Mutex::new(Vec::new()),
    };
    let vehicle =
        VehicleImplementation::read(&recording, "TEST_Ship", "Scripts/Other/Foo.xml", None)?;
    assert!(vehicle.path().eq_ignore_ascii_case(DEFINITION));
    let none = VehicleImplementation::read(&recording, "Other_Ship", "Scripts/Other/Foo.xml", None);
    assert!(
        matches!(&none, Err(error) if error.kind() == "vehicle_implementation"),
        "{none:?}"
    );
    Ok(())
}

#[test]
fn numbers_take_the_longest_leading_numeric_prefix() -> TestResult {
    let cases = [
        ("1500", Some(1500.0)),
        (" 12.5kg ", Some(12.5)),
        ("1,5", Some(1.0)),
        ("v2 1500", None),
        ("n/a", None),
        ("-3e2x", Some(-300.0)),
        ("1e", Some(1.0)),
        (".5", Some(0.5)),
        ("5.", Some(5.0)),
        ("-", None),
        ("", None),
        ("1e999", None),
    ];
    let parts = cases.iter().fold(String::new(), |mut out, (mass, _)| {
        let _ = write!(out, r#"<Part name="p" mass="{mass}"/>"#);
        out
    });
    let files = files(&[(DEFINITION, patched_base("", &parts).as_str())]);
    let vehicle = VehicleImplementation::read(&files, "TEST_Ship", DEFINITION, None)?;
    let read: Vec<_> = vehicle
        .parts()
        .iter()
        .map(p4kx_assembly::VehiclePart::mass)
        .collect();
    let wanted: Vec<_> = cases.iter().map(|(_, mass)| *mass).collect();
    assert_eq!(read, wanted);
    let raw = vehicle
        .parts()
        .get(2)
        .and_then(|part| part.attribute("MASS"));
    assert_eq!(raw, Some("1,5"), "the raw value stays available");
    Ok(())
}
