// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cmp::Ordering;

use p4kx_extraction::{DataForge, Enum, Filter, Property, Struct};

use crate::budget::fill;
use crate::error::ToolError;
use crate::forge_shapes::{EnumShape, EnumsResult, FieldShape, StructShape, StructsResult};
use crate::page::Page;

/// Returns one page of the struct definitions whose name matches the Filter `name`, by name
/// ignoring ASCII case.
pub(crate) fn structs(
    forge: &DataForge<'_>,
    archive: String,
    name: Option<&str>,
    page: Page,
) -> Result<StructsResult, ToolError> {
    let named = matching(forge.structs(), name, Struct::name)?;
    let items = page
        .slice(&named)
        .iter()
        .map(|(name, definition)| struct_shape(name, definition));
    let (structs, next_cursor) = fill(page, named.len(), items, |shape| {
        format!("struct {}", shape.name)
    })?;
    Ok(StructsResult {
        archive,
        total: named.len(),
        structs,
        next_cursor,
    })
}

/// Returns one page of the enum definitions whose name matches the Filter `name`, by name
/// ignoring ASCII case.
pub(crate) fn enums(
    forge: &DataForge<'_>,
    archive: String,
    name: Option<&str>,
    page: Page,
) -> Result<EnumsResult, ToolError> {
    let named = matching(forge.enums(), name, Enum::name)?;
    let items = page.slice(&named).iter().map(|(name, definition)| {
        Ok(EnumShape {
            name: (*name).to_owned(),
            values: definition
                .options()
                .map(|option| option.map(str::to_owned))
                .collect::<Result<_, _>>()?,
        })
    });
    let (enums, next_cursor) = fill(page, named.len(), items, |shape| {
        format!("enum {}", shape.name)
    })?;
    Ok(EnumsResult {
        archive,
        total: named.len(),
        enums,
        next_cursor,
    })
}

/// Returns the definitions whose name matches `filter`, with their names, by name ignoring
/// ASCII case and then in definition order.
fn matching<'f, T>(
    definitions: impl Iterator<Item = T>,
    filter: Option<&str>,
    name: impl Fn(&T) -> Result<&'f str, p4kx_extraction::DataForgeError>,
) -> Result<Vec<(&'f str, T)>, ToolError> {
    let filter = filter.map(Filter::parse).transpose()?;
    let mut named = Vec::new();
    for definition in definitions {
        let text = name(&definition)?;
        if filter.as_ref().is_none_or(|filter| filter.matches(text)) {
            named.push((text, definition));
        }
    }
    named.sort_by(|left, right| compare_ignoring_case(left.0, right.0));
    Ok(named)
}

fn struct_shape(name: &str, definition: &Struct<'_>) -> Result<StructShape, ToolError> {
    Ok(StructShape {
        name: name.to_owned(),
        parent: definition
            .parent()
            .map(|parent| parent.name().map(str::to_owned))
            .transpose()?,
        fields: definition
            .properties()
            .map(|property| field_shape(&property))
            .collect::<Result<_, _>>()?,
    })
}

fn field_shape(property: &Property<'_>) -> Result<FieldShape, ToolError> {
    Ok(FieldShape {
        name: property.name()?.to_owned(),
        data_type: property.data_type()?.to_string(),
        array: property.is_array(),
        target: property
            .target_struct()
            .map(|target| target.name().map(str::to_owned))
            .transpose()?,
        enum_name: property
            .target_enum()
            .map(|target| target.name().map(str::to_owned))
            .transpose()?,
    })
}

fn compare_ignoring_case(left: &str, right: &str) -> Ordering {
    let key = |byte: u8| byte.to_ascii_lowercase();
    left.bytes().map(key).cmp(right.bytes().map(key))
}
