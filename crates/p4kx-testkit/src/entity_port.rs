// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{TestInstance, TestValue};

/// An item port definition of a synthetic Entity.
#[derive(Debug, Clone, PartialEq)]
pub struct TestPort {
    name: String,
    sizes: (u32, u32),
    types: Vec<(String, Vec<String>)>,
    helper: Option<(String, [f32; 3], [f32; 3])>,
    attachment: &'static str,
    default_item: Option<u128>,
    flags: String,
    tags: (String, String),
}

impl TestPort {
    /// Creates a port of size 1 without types, helper and default item.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sizes: (1, 1),
            types: Vec::new(),
            helper: None,
            attachment: "Bone",
            default_item: None,
            flags: String::new(),
            tags: (String::new(), String::new()),
        }
    }

    /// Sets the flags of the port, words separated by white space.
    #[must_use]
    pub fn flags(mut self, flags: impl Into<String>) -> Self {
        self.flags = flags.into();
        self
    }

    /// Sets the tags the port offers and the tags an item needs to fit it, words separated by
    /// white space.
    #[must_use]
    pub fn tags(mut self, port_tags: impl Into<String>, required: impl Into<String>) -> Self {
        self.tags = (port_tags.into(), required.into());
        self
    }

    /// Sets the smallest and largest item size the port takes.
    #[must_use]
    pub const fn sizes(mut self, min: u32, max: u32) -> Self {
        self.sizes = (min, max);
        self
    }

    /// Adds an item type the port takes, with its sub-types.
    #[must_use]
    pub fn item_type<S: Into<String>>(
        mut self,
        name: impl Into<String>,
        sub_types: impl IntoIterator<Item = S>,
    ) -> Self {
        self.types
            .push((name.into(), sub_types.into_iter().map(Into::into).collect()));
        self
    }

    /// Attaches the port to the bone helper `name`, offset by `position` and `rotation`.
    #[must_use]
    pub fn helper(
        mut self,
        name: impl Into<String>,
        position: [f32; 3],
        rotation: [f32; 3],
    ) -> Self {
        self.helper = Some((name.into(), position, rotation));
        self
    }

    /// Attaches the port as a skin to the helper `name`, offset by `position` and `rotation`.
    #[must_use]
    pub fn skin_helper(
        mut self,
        name: impl Into<String>,
        position: [f32; 3],
        rotation: [f32; 3],
    ) -> Self {
        self.helper = Some((name.into(), position, rotation));
        self.attachment = "Skin";
        self
    }

    /// Sets the Entity the port holds when no Loadout names one.
    #[must_use]
    pub const fn default_item(mut self, guid: u128) -> Self {
        self.default_item = Some(guid);
        self
    }

    pub(crate) fn value(&self) -> TestValue {
        let mut instance = TestInstance::new("SItemPortDef")
            .value("Name", TestValue::text(self.name.clone()))
            .value("DisplayName", TestValue::text("@LOC_EMPTY"))
            .value("Flags", TestValue::text(self.flags.clone()))
            .value("PortTags", TestValue::text(self.tags.0.clone()))
            .value("RequiredPortTags", TestValue::text(self.tags.1.clone()))
            .value("MinSize", TestValue::UInt32(self.sizes.0))
            .value("MaxSize", TestValue::UInt32(self.sizes.1))
            .value(
                "Types",
                TestValue::Array(
                    self.types
                        .iter()
                        .map(|(name, sub_types)| {
                            TestValue::Struct(
                                TestInstance::new("SItemPortDefTypes")
                                    .value("Type", TestValue::text(name.clone()))
                                    .value(
                                        "SubTypes",
                                        TestValue::Array(
                                            sub_types
                                                .iter()
                                                .cloned()
                                                .map(TestValue::Text)
                                                .collect(),
                                        ),
                                    ),
                            )
                        })
                        .collect(),
                ),
            );
        if let Some(guid) = self.default_item {
            instance = instance.value(
                "defaultItem",
                TestValue::Struct(
                    TestInstance::new("SItemPortDefaultItemDef")
                        .value("entityClass", TestValue::Reference(guid)),
                ),
            );
        }
        if let Some((name, position, rotation)) = &self.helper {
            instance = instance.value(
                "AttachmentImplementation",
                helper(self.attachment, name, *position, *rotation),
            );
        }
        TestValue::Struct(instance)
    }
}

fn helper(attachment: &str, name: &str, position: [f32; 3], rotation: [f32; 3]) -> TestValue {
    let xyz = |type_name: &str, [x, y, z]: [f32; 3]| {
        TestValue::Struct(
            TestInstance::new(type_name)
                .value("x", TestValue::Single(x))
                .value("y", TestValue::Single(y))
                .value("z", TestValue::Single(z)),
        )
    };
    let offset = TestInstance::new("QuatT")
        .value("Rotation", xyz("Ang3", rotation))
        .value("Position", xyz("Vec3", position));
    let helper = TestInstance::new("SItemPortDefHelper")
        .value("Name", TestValue::text(name))
        .value("ItemOffsetHelperName", TestValue::text(""))
        .value("Offset", TestValue::Struct(offset));
    let node =
        TestInstance::new("SItemPortDefHelperNode").value("Helper", TestValue::Struct(helper));
    TestValue::Struct(
        TestInstance::new(format!("SItemPortDefAttachmentImplementation{attachment}"))
            .value("Helper", TestValue::Struct(node)),
    )
}
