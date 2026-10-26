// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;

use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};

/// The output-only name that [`crate::Chf::to_json_named`] writes beside a hash.
///
/// Reading accepts the key and drops its text; writing never emits it, and two values that differ
/// only in it are equal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Label;

struct LabelVisitor;

impl Visitor<'_> for LabelVisitor {
    type Value = Label;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a name as a string")
    }

    fn visit_str<E: Error>(self, _name: &str) -> Result<Label, E> {
        Ok(Label)
    }
}

impl<'de> Deserialize<'de> for Label {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(LabelVisitor)
    }
}
