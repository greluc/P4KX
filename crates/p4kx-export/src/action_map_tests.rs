// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement};

use crate::action_map::{ActionMaps, Device, parse_action_maps};

const PROFILE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<profile version="1">
  <actiongroup action="v_attack"><action name="v_attack_all"/></actiongroup>
  <actionmap name="seat_general" version="1" UILabel="@ui_CGSeatGeneral">
    <action name="v_eject" activationMode="press" keyboard="ralt+y" gamepad=" " joystick=" " UILabel="@ui_CIEject" UIDescription="@ui_CIEjectDesc"/>
    <action name="v_strafe" activationMode="hold" keyboard="mouse1" mouse="mouse1" gamepad="triggerr_btn" joystick="button1" UILabel="@ui_Strafe"/>
    <action name="prone" activationMode="press" keyboard="lctrl" UILabel="@ui_Prone">
      <gamepad activationMode="delayed_press" input="b"/>
      <keyboard activationMode="double_tap" input="q"/>
      <keyboard><inputdata input="enter"/><inputdata input="np_enter"/></keyboard>
    </action>
    <action name="same" activationMode="press" gamepad="back"><gamepad input="back"/></action>
    <action name="blank" keyboard="" UILabel=""/>
  </actionmap>
  <actionmap name="hacking" UILable="@ui_Hack"><action name="h"/></actionmap>
</profile>"#;

fn parsed() -> ActionMaps {
    parse_action_maps(PROFILE.as_bytes()).expect("a profile")
}

fn inputs(maps: &ActionMaps, action: &str, device: Device) -> Vec<(String, Option<String>)> {
    maps.maps
        .iter()
        .flat_map(|map| &map.actions)
        .find(|candidate| candidate.name == action)
        .expect("the action")
        .bindings
        .iter()
        .filter(|binding| binding.device == device)
        .map(|binding| (binding.input.clone(), binding.activation_mode.clone()))
        .collect()
}

fn pair(input: &str, mode: &str) -> (String, Option<String>) {
    (input.to_owned(), Some(mode.to_owned()))
}

#[test]
fn only_actions_inside_an_action_map_are_read() {
    let maps = parsed();
    let names: Vec<&str> = maps.maps.iter().map(|map| map.name.as_str()).collect();
    assert_eq!(names, ["seat_general", "hacking"]);
    assert_eq!(maps.action_count(), 6);
    assert!(
        maps.maps
            .iter()
            .flat_map(|map| &map.actions)
            .all(|action| action.name != "v_attack_all")
    );
}

#[test]
fn labels_come_from_ui_label_and_the_misspelt_ui_lable() {
    let maps = parsed();
    assert_eq!(maps.maps[0].label_key.as_deref(), Some("@ui_CGSeatGeneral"));
    assert_eq!(maps.maps[1].label_key.as_deref(), Some("@ui_Hack"));
    assert_eq!(
        maps.maps[0].actions[0].label_key.as_deref(),
        Some("@ui_CIEject")
    );
    assert_eq!(maps.maps[0].actions[4].label_key, None);
}

#[test]
fn a_blank_input_is_no_binding() {
    let maps = parsed();
    assert_eq!(
        inputs(&maps, "v_eject", Device::Keyboard),
        [pair("ralt+y", "press")]
    );
    assert_eq!(inputs(&maps, "v_eject", Device::Gamepad).len(), 0);
    assert_eq!(inputs(&maps, "v_eject", Device::Joystick).len(), 0);
    assert_eq!(inputs(&maps, "blank", Device::Keyboard).len(), 0);
}

#[test]
fn every_device_attribute_is_a_binding_of_its_device() {
    let maps = parsed();
    assert_eq!(
        inputs(&maps, "v_strafe", Device::Keyboard),
        [pair("mouse1", "hold")]
    );
    assert_eq!(
        inputs(&maps, "v_strafe", Device::Mouse),
        [pair("mouse1", "hold")]
    );
    assert_eq!(
        inputs(&maps, "v_strafe", Device::Gamepad),
        [pair("triggerr_btn", "hold")]
    );
    assert_eq!(
        inputs(&maps, "v_strafe", Device::Joystick),
        [pair("button1", "hold")]
    );
}

#[test]
fn device_children_add_bindings_with_their_own_activation_mode() {
    let maps = parsed();
    assert_eq!(
        inputs(&maps, "prone", Device::Keyboard),
        [
            pair("lctrl", "press"),
            pair("q", "double_tap"),
            pair("enter", "press"),
            pair("np_enter", "press"),
        ]
    );
    assert_eq!(
        inputs(&maps, "prone", Device::Gamepad),
        [pair("b", "delayed_press")]
    );
}

#[test]
fn a_binding_given_twice_is_listed_once() {
    let maps = parsed();
    assert_eq!(
        inputs(&maps, "same", Device::Gamepad),
        [pair("back", "press")]
    );
}

#[test]
fn cryxml_reads_like_text_xml() {
    let root = TestElement::new("profile").child(
        TestElement::new("actionmap")
            .attribute("name", "m")
            .attribute("UILabel", "@ui_M")
            .child(
                TestElement::new("action")
                    .attribute("name", "a")
                    .attribute("activationMode", "tap")
                    .attribute("keyboard", "u+lshift")
                    .child(TestElement::new("joystick").attribute("input", "button3")),
            ),
    );
    let bytes = root.to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
    let maps = parse_action_maps(&bytes).expect("a profile");
    assert_eq!(
        inputs(&maps, "a", Device::Keyboard),
        [pair("u+lshift", "tap")]
    );
    assert_eq!(
        inputs(&maps, "a", Device::Joystick),
        [pair("button3", "tap")]
    );
}

#[test]
fn bytes_that_are_no_xml_are_an_error() {
    assert!(parse_action_maps(b"\x00\x01 not xml").is_err());
    assert!(parse_action_maps(b"<profile><actionmap>").is_err());
}
