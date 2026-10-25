// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{TestInstance, TestStruct, TestType, TestValue};

/// One zone of the body a worn Entity covers, as the `Chunks` of `SCItemClothingParams` hold it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestZoneChunk {
    /// The zone's name, e.g. `torso01_zone`.
    pub zone: String,
    /// The layer the zone is covered on; the body is on 0.
    pub layer: i32,
    /// The lower layers that stay visible through the chunk.
    pub visible_layers: Vec<i32>,
    /// Whether the chunk has a visibility condition.
    pub conditional: bool,
}

impl TestZoneChunk {
    /// Creates a chunk of `zone` on `layer` that lets nothing below show through.
    #[must_use]
    pub fn new(zone: impl Into<String>, layer: i32) -> Self {
        Self {
            zone: zone.into(),
            layer,
            visible_layers: Vec::new(),
            conditional: false,
        }
    }

    /// Lets the `layers` below show through the chunk.
    #[must_use]
    pub fn showing(mut self, layers: &[i32]) -> Self {
        self.visible_layers = layers.to_vec();
        self
    }

    /// Gives the chunk a visibility condition.
    #[must_use]
    pub const fn conditional(mut self) -> Self {
        self.conditional = true;
        self
    }

    fn value(&self) -> TestValue {
        let conditions = if self.conditional {
            vec![TestValue::Struct(TestInstance::new(
                "SCItemClothingVisibilityConditionParams",
            ))]
        } else {
            Vec::new()
        };
        TestValue::Struct(
            TestInstance::new("SCItemClothingMeshChunkParams")
                .value("MeshChunk", TestValue::text(self.zone.clone()))
                .value("Layer", TestValue::Int32(self.layer))
                .value(
                    "VisibleLayers",
                    TestValue::Array(
                        self.visible_layers
                            .iter()
                            .map(|layer| TestValue::Int32(*layer))
                            .collect(),
                    ),
                )
                .value("VisibilityConditions", TestValue::Array(conditions)),
        )
    }
}

/// Returns the `SCItemClothingParams` component that covers `chunks`; add it to an Entity with
/// [`TestEntity::component`](crate::TestEntity::component).
#[must_use]
pub fn clothing_component(chunks: &[TestZoneChunk]) -> TestValue {
    TestValue::Struct(TestInstance::new("SCItemClothingParams").value(
        "Chunks",
        TestValue::Array(chunks.iter().map(TestZoneChunk::value).collect()),
    ))
}

pub(crate) fn clothing_structs() -> Vec<TestStruct> {
    vec![
        TestStruct::new("SCItemClothingParams")
            .parent("DataForgeComponentParams")
            .array(
                "Chunks",
                TestType::Class("SCItemClothingMeshChunkParams".to_owned()),
            ),
        TestStruct::new("SCItemClothingMeshChunkParams")
            .field("MeshChunk", TestType::String)
            .field("Layer", TestType::Int32)
            .array("VisibleLayers", TestType::Int32)
            .array(
                "VisibilityConditions",
                TestType::Class("SCItemClothingVisibilityConditionParams".to_owned()),
            ),
        TestStruct::new("SCItemClothingVisibilityConditionParams"),
    ]
}
