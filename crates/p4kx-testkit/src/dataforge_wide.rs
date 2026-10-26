// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{DataForgeBuilder, TestInstance, TestRecord, TestStruct, TestType, TestValue};

/// Builds a synthetic DataForge whose Records have `types * types * fields` distinct field paths.
///
/// It has `types` structs of `fields` `Int32` fields each, no two fields of the database sharing
/// a name. Each of `types` Records owns an instance of one struct that owns an instance of every
/// struct, so each pair of structs gives `fields` paths. No Record has more than
/// `types * fields` values, and the file holds `types * fields` properties, so both stay within
/// the limits of the format when `types * fields` is below 65,000.
#[must_use]
pub fn many_paths_database(types: usize, fields: usize) -> Vec<u8> {
    let mut builder = DataForgeBuilder::new()
        .structure(
            TestStruct::new("Base").array("kids", TestType::StrongPointer("Base".to_owned())),
        )
        .structure(
            TestStruct::new("Holder").array("items", TestType::StrongPointer("Base".to_owned())),
        );
    for index in 0..types {
        let mut structure = TestStruct::new(format!("Kind{index}")).parent("Base");
        for field in 0..fields {
            structure = structure.field(format!("k{index}_{field}"), TestType::Int32);
        }
        builder = builder.structure(structure);
    }
    let kids = TestValue::Array(
        (0..types)
            .map(|kind| TestValue::Struct(TestInstance::new(format!("Kind{kind}"))))
            .collect(),
    );
    for index in 0..types {
        let owner = TestInstance::new(format!("Kind{index}")).value("kids", kids.clone());
        builder = builder.record(
            TestRecord::new(
                index as u128 + 1,
                "Holder",
                format!("Holder.H{index}"),
                format!("holders/h{index}.xml"),
            )
            .value("items", TestValue::Array(vec![TestValue::Struct(owner)])),
        );
    }
    builder.build()
}
