// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The text of a node as a test reads it, shared by the test binaries that include this file.

use egui::accesskit::Role;
use egui_kittest::Node;
use egui_kittest::kittest::NodeT;

/// The text of `node`: kittest keeps it in `value` for a `Role::Label` node and in `label` for
/// any other role.
pub(crate) fn node_text(node: &Node<'_>) -> Option<String> {
    let node = node.accesskit_node();
    if node.role() == Role::Label {
        node.value()
    } else {
        node.label()
    }
}
