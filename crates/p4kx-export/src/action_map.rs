// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Read};

use p4kx_extraction::cryxml::{self, CryXmlError, Xml};
use p4kx_extraction::{Archive, find_entry};
use thiserror::Error;

use crate::starmap_xml::{Node, attribute};

/// The path of the default profile that holds the action maps.
pub const ACTION_MAP_PROFILE_PATH: &str = "Data\\Libs\\Config\\defaultProfile.xml";

const MAX_PROFILE_LEN: u64 = 64 << 20;
const DEVICES: [Device; 4] = [
    Device::Keyboard,
    Device::Mouse,
    Device::Gamepad,
    Device::Joystick,
];

/// An input device a default binding belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Device {
    /// The keyboard.
    Keyboard,
    /// The mouse.
    Mouse,
    /// The gamepad.
    Gamepad,
    /// The joystick.
    Joystick,
}

impl Device {
    /// Returns the name the profile and the exports use for the device.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Keyboard => "keyboard",
            Self::Mouse => "mouse",
            Self::Gamepad => "gamepad",
            Self::Joystick => "joystick",
        }
    }
}

/// One default input of an action on one device, such as `lalt+mouse1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The device the input belongs to.
    pub device: Device,
    /// The input as the profile writes it; `+` joins the keys of a combination.
    pub input: String,
    /// The activation mode of this binding: its own, else the one of the action.
    pub activation_mode: Option<String>,
}

/// An action of an action map with its default bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// The action name, such as `v_toggle_mining_mode`.
    pub name: String,
    /// The localisation key of the label, with its leading `@`; `None` when the action has none.
    pub label_key: Option<String>,
    /// The activation mode the action names.
    pub activation_mode: Option<String>,
    /// The default bindings that are set, in the order keyboard, mouse, gamepad, joystick and
    /// within a device in document order.
    pub bindings: Vec<Binding>,
}

/// An action map: a named group of actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionMap {
    /// The action map name, such as `spaceship_general`.
    pub name: String,
    /// The localisation key of the label, with its leading `@`; `None` when the map has none.
    pub label_key: Option<String>,
    /// The actions in document order.
    pub actions: Vec<Action>,
}

/// The action maps of a default profile, in document order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionMaps {
    /// The action maps.
    pub maps: Vec<ActionMap>,
}

impl ActionMaps {
    /// Returns the number of actions in all maps.
    #[must_use]
    pub fn action_count(&self) -> usize {
        self.maps.iter().map(|map| map.actions.len()).sum()
    }
}

/// The action maps cannot be read.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ActionMapError {
    /// The Archive holds no default profile.
    #[error("the Archive holds no {ACTION_MAP_PROFILE_PATH}")]
    NotFound,
    /// The default profile is larger than the limit.
    #[error("{ACTION_MAP_PROFILE_PATH} holds {size} bytes, over the limit of {MAX_PROFILE_LEN}")]
    TooLarge {
        /// The size of the Entry in bytes.
        size: u64,
    },
    /// The default profile cannot be opened or read.
    #[error("cannot read {ACTION_MAP_PROFILE_PATH}")]
    Read(#[source] io::Error),
    /// The default profile is neither text XML nor `CryXml`.
    #[error("{ACTION_MAP_PROFILE_PATH} is not valid XML")]
    Xml(#[source] CryXmlError),
    /// The default profile is text XML that does not parse.
    #[error("{ACTION_MAP_PROFILE_PATH} is not well-formed text XML: {0}")]
    Text(String),
}

/// Reads the action maps of the default profile of `archive`.
///
/// # Errors
///
/// Returns [`ActionMapError`] when the profile is missing, too large, unreadable or not XML.
pub fn read_action_maps(archive: &Archive) -> Result<ActionMaps, ActionMapError> {
    let entry = find_entry(archive, ACTION_MAP_PROFILE_PATH).ok_or(ActionMapError::NotFound)?;
    let size = entry.uncompressed_size();
    if size > MAX_PROFILE_LEN {
        return Err(ActionMapError::TooLarge { size });
    }
    let mut bytes = Vec::new();
    archive
        .open_entry(entry)
        .map_err(|error| ActionMapError::Read(io::Error::other(error)))?
        .take(MAX_PROFILE_LEN)
        .read_to_end(&mut bytes)
        .map_err(ActionMapError::Read)?;
    parse_action_maps(&bytes)
}

/// Parses the action maps of the profile `bytes`, text XML or `CryXml`.
///
/// Only the actions inside an `actionmap` element count. An input that is empty or blank is no
/// binding, and the same device, input and activation mode of one action appear once.
///
/// # Errors
///
/// Returns [`ActionMapError::Xml`] or [`ActionMapError::Text`] when `bytes` are no XML.
pub fn parse_action_maps(bytes: &[u8]) -> Result<ActionMaps, ActionMapError> {
    match cryxml::read(bytes).map_err(ActionMapError::Xml)? {
        Xml::CryXml(document) => Ok(collect(document.root())),
        Xml::Text(text) => {
            let text = std::str::from_utf8(text)
                .map_err(|error| ActionMapError::Text(error.to_string()))?
                .trim_start_matches('\u{feff}');
            cryxml::check_text_depth(text)
                .map_err(|error| ActionMapError::Text(error.to_string()))?;
            let document = roxmltree::Document::parse(text)
                .map_err(|error| ActionMapError::Text(error.to_string()))?;
            Ok(collect(document.root_element()))
        }
    }
}

fn named<'d, N: Node<'d>>(node: N, name: &'static str) -> impl Iterator<Item = N> {
    node.children()
        .filter(move |child| child.name().eq_ignore_ascii_case(name))
}

fn text<'d>(node: impl Node<'d>, name: &str) -> Option<&'d str> {
    attribute(node, name)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn collect<'d, N: Node<'d>>(root: N) -> ActionMaps {
    let maps = named(root, "actionmap")
        .map(|map| ActionMap {
            name: text(map, "name").unwrap_or_default().to_owned(),
            label_key: text(map, "UILabel")
                .or_else(|| text(map, "UILable"))
                .map(str::to_owned),
            actions: named(map, "action").filter_map(read_action).collect(),
        })
        .collect();
    ActionMaps { maps }
}

fn read_action<'d>(action: impl Node<'d>) -> Option<Action> {
    let name = text(action, "name")?.to_owned();
    let activation_mode = text(action, "activationMode");
    let mut bindings = Vec::new();
    for device in DEVICES {
        let mut push = |input: &str, mode: Option<&str>| {
            let binding = Binding {
                device,
                input: input.to_owned(),
                activation_mode: mode.or(activation_mode).map(str::to_owned),
            };
            if !bindings.contains(&binding) {
                bindings.push(binding);
            }
        };
        if let Some(input) = text(action, device.name()) {
            push(input, None);
        }
        for element in named(action, device.name()) {
            let mode = text(element, "activationMode");
            if let Some(input) = text(element, "input") {
                push(input, mode);
            }
            for data in named(element, "inputdata") {
                if let Some(input) = text(data, "input") {
                    push(input, mode);
                }
            }
        }
    }
    Some(Action {
        name,
        label_key: text(action, "UILabel").map(str::to_owned),
        activation_mode: activation_mode.map(str::to_owned),
        bindings,
    })
}
