// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use thiserror::Error;

use crate::{PortDefinition, VehicleItemPort};

/// What an Entity offers a port it is attached to: its item type, sub-type and size
/// (`AttachDef.Type`, `AttachDef.SubType`, `AttachDef.Size`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFacts {
    pub(crate) class_name: String,
    pub(crate) item_type: String,
    pub(crate) sub_type: String,
    pub(crate) size: Option<u32>,
}

impl ItemFacts {
    /// Returns the class name of the Entity.
    #[must_use]
    pub fn class_name(&self) -> &str {
        &self.class_name
    }

    /// Returns the item type, e.g. `WeaponGun`; empty when the Entity has none.
    #[must_use]
    pub fn item_type(&self) -> &str {
        &self.item_type
    }

    /// Returns the item sub-type, e.g. `Gun`; empty when the Entity has none.
    #[must_use]
    pub fn sub_type(&self) -> &str {
        &self.sub_type
    }

    /// Returns the item size, or `None` when the Entity gives none.
    #[must_use]
    pub const fn size(&self) -> Option<u32> {
        self.size
    }
}

/// An item type a port takes with the sub-types it takes of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedType {
    pub(crate) name: String,
    pub(crate) sub_types: Vec<String>,
}

impl AcceptedType {
    /// Returns the item type, e.g. `WeaponGun`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the sub-types taken; empty when every sub-type is.
    #[must_use]
    pub fn sub_types(&self) -> &[String] {
        &self.sub_types
    }

    fn text(&self) -> String {
        if self.sub_types.is_empty() {
            self.name.clone()
        } else {
            format!("{} ({})", self.name, self.sub_types.join(", "))
        }
    }
}

/// What a port takes: the item types and the range of item sizes.
///
/// It comes from the port's definition in the Entity Record or, for a port the Record does not
/// define, from the item port of the vehicle implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortConstraints {
    pub(crate) min_size: Option<u32>,
    pub(crate) max_size: Option<u32>,
    pub(crate) types: Vec<AcceptedType>,
}

impl PortConstraints {
    pub(crate) fn of_definition(definition: &PortDefinition) -> Option<Self> {
        let constraints = Self {
            min_size: definition.min_size(),
            max_size: stated_max(definition.min_size(), definition.max_size()),
            types: definition
                .types()
                .iter()
                .map(|item_type| AcceptedType {
                    name: item_type.name().to_owned(),
                    sub_types: item_type.sub_types().to_vec(),
                })
                .collect(),
        };
        constraints.is_set().then_some(constraints)
    }

    pub(crate) fn of_vehicle(port: &VehicleItemPort) -> Option<Self> {
        let constraints = Self {
            min_size: port.min_size(),
            max_size: stated_max(port.min_size(), port.max_size()),
            types: port
                .item_types()
                .iter()
                .map(|kind| AcceptedType {
                    name: kind.item_type().to_owned(),
                    sub_types: kind.subtypes().to_vec(),
                })
                .collect(),
        };
        constraints.is_set().then_some(constraints)
    }

    const fn is_set(&self) -> bool {
        self.min_size.is_some() || self.max_size.is_some() || !self.types.is_empty()
    }

    /// Returns the smallest item size the port takes, if it states one.
    #[must_use]
    pub const fn min_size(&self) -> Option<u32> {
        self.min_size
    }

    /// Returns the largest item size the port takes, if it states one; a largest size of 0 beside
    /// a smallest size of 0 or none is the unauthored default and counts as none.
    #[must_use]
    pub const fn max_size(&self) -> Option<u32> {
        self.max_size
    }

    /// Returns the item types the port takes; empty when it takes every type.
    #[must_use]
    pub fn types(&self) -> &[AcceptedType] {
        &self.types
    }

    /// Checks that an item of `item` fits the port.
    ///
    /// An item type is matched ignoring ASCII case, and a type that names sub-types takes only
    /// those, except that an item whose sub-type is empty or `UNDEFINED` fits any of them. An
    /// item that gives no size, or size 0, is not held against the sizes.
    ///
    /// # Errors
    ///
    /// Returns the first constraint the item breaks: its type, its sub-type, then its size.
    pub fn check(&self, item: &ItemFacts) -> Result<(), Incompatibility> {
        let sub_type_open =
            item.sub_type.is_empty() || item.sub_type.eq_ignore_ascii_case("UNDEFINED");
        if !self.types.is_empty() {
            let named: Vec<&AcceptedType> = self
                .types
                .iter()
                .filter(|accepted| accepted.name.eq_ignore_ascii_case(&item.item_type))
                .collect();
            if named.is_empty() {
                return Err(Incompatibility::Type {
                    item_type: item.item_type.clone(),
                    accepted: self.types.iter().map(|t| t.name.clone()).collect(),
                });
            }
            let fits = named.iter().any(|accepted| {
                sub_type_open
                    || accepted.sub_types.is_empty()
                    || accepted
                        .sub_types
                        .iter()
                        .any(|sub_type| sub_type.eq_ignore_ascii_case(&item.sub_type))
            });
            if !fits {
                return Err(Incompatibility::SubType {
                    item_type: item.item_type.clone(),
                    sub_type: item.sub_type.clone(),
                    accepted: named.iter().map(|accepted| accepted.text()).collect(),
                });
            }
        }
        if let Some(size) = item.size.filter(|size| *size > 0) {
            if let Some(min) = self.min_size
                && size < min
            {
                return Err(Incompatibility::TooSmall { size, min });
            }
            if let Some(max) = self.max_size
                && size > max
            {
                return Err(Incompatibility::TooLarge { size, max });
            }
        }
        Ok(())
    }
}

/// Why an Entity does not fit a port.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum Incompatibility {
    /// The item type is none the port takes.
    #[error(
        "its item type is {}, the port takes {}",
        shown(item_type),
        accepted.join(", ")
    )]
    Type {
        /// The Entity's item type; empty when it has none.
        item_type: String,
        /// The item types the port takes.
        accepted: Vec<String>,
    },
    /// The item type fits, its sub-type does not.
    #[error(
        "its item type {item_type} has the sub-type {}, the port takes {}",
        shown(sub_type),
        accepted.join(", ")
    )]
    SubType {
        /// The Entity's item type.
        item_type: String,
        /// The Entity's sub-type; empty when it has none.
        sub_type: String,
        /// The types the port takes of that item type, with their sub-types.
        accepted: Vec<String>,
    },
    /// The item is smaller than the port takes.
    #[error("its size {size} is below the smallest size {min} the port takes")]
    TooSmall {
        /// The item's size.
        size: u32,
        /// The smallest size the port takes.
        min: u32,
    },
    /// The item is larger than the port takes.
    #[error("its size {size} is above the largest size {max} the port takes")]
    TooLarge {
        /// The item's size.
        size: u32,
        /// The largest size the port takes.
        max: u32,
    },
}

impl Incompatibility {
    /// Returns a stable code for the kind of mismatch, e.g. `too_large`.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Type { .. } => "item_type",
            Self::SubType { .. } => "item_sub_type",
            Self::TooSmall { .. } => "too_small",
            Self::TooLarge { .. } => "too_large",
        }
    }
}

const fn stated_max(min: Option<u32>, max: Option<u32>) -> Option<u32> {
    match (min, max) {
        (None | Some(0), Some(0)) => None,
        _ => max,
    }
}

const fn shown(text: &str) -> &str {
    if text.is_empty() { "none" } else { text }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gun(sub_type: &str) -> ItemFacts {
        ItemFacts {
            class_name: "GUN".to_owned(),
            item_type: "WeaponGun".to_owned(),
            sub_type: sub_type.to_owned(),
            size: None,
        }
    }

    fn port() -> PortConstraints {
        PortConstraints {
            min_size: None,
            max_size: None,
            types: vec![AcceptedType {
                name: "WeaponGun".to_owned(),
                sub_types: vec!["Gun".to_owned()],
            }],
        }
    }

    #[test]
    fn an_empty_or_undefined_sub_type_fits_any_listed_one() {
        assert_eq!(port().check(&gun("")), Ok(()));
        assert_eq!(port().check(&gun("UNDEFINED")), Ok(()));
        assert_eq!(port().check(&gun("undefined")), Ok(()));
        assert_eq!(port().check(&gun("Gun")), Ok(()));
    }

    fn sized(min: Option<u32>, max: Option<u32>, size: u32) -> Result<(), Incompatibility> {
        let mut item = gun("Gun");
        item.size = Some(size);
        PortConstraints {
            min_size: min,
            max_size: stated_max(min, max),
            types: Vec::new(),
        }
        .check(&item)
    }

    #[test]
    fn a_largest_size_of_zero_with_a_smallest_size_of_zero_or_none_states_no_limit() {
        assert_eq!(sized(Some(0), Some(0), 1), Ok(()));
        assert_eq!(sized(None, Some(0), 4), Ok(()));
        assert_eq!(sized(Some(0), Some(0), 0), Ok(()));
        assert_eq!(stated_max(Some(0), Some(0)), None);
        assert_eq!(stated_max(Some(0), Some(3)), Some(3));
    }

    #[test]
    fn an_item_of_size_zero_is_not_held_against_the_sizes() {
        assert_eq!(sized(Some(1), Some(1), 0), Ok(()));
        assert_eq!(sized(Some(2), Some(3), 0), Ok(()));
    }

    #[test]
    fn a_stated_range_is_still_enforced() {
        assert_eq!(
            sized(Some(1), Some(1), 2),
            Err(Incompatibility::TooLarge { size: 2, max: 1 })
        );
        assert_eq!(
            sized(Some(2), Some(2), 1),
            Err(Incompatibility::TooSmall { size: 1, min: 2 })
        );
        assert_eq!(
            sized(Some(0), Some(3), 4),
            Err(Incompatibility::TooLarge { size: 4, max: 3 })
        );
    }

    #[test]
    fn another_sub_type_is_still_rejected() {
        assert!(matches!(
            port().check(&gun("Missile")),
            Err(Incompatibility::SubType { .. })
        ));
    }
}
