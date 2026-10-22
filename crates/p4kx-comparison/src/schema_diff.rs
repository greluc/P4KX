use std::collections::BTreeMap;
use std::fmt;

use schemars::JsonSchema;
use serde::Serialize;

use crate::DataForge;
use crate::schema_side::{EnumDef, SideSchema, StructDef};

/// How a struct, property, enum or enum option differs between the two sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchemaChangeKind {
    /// Only the new side has it.
    Added,
    /// Only the old side has it.
    Removed,
    /// Both sides have it and it differs.
    Changed,
}

impl fmt::Display for SchemaChangeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Changed => "changed",
        })
    }
}

/// A count for each side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, JsonSchema)]
pub struct SideCounts {
    /// The count on the old side.
    pub old: u64,
    /// The count on the new side.
    pub new: u64,
}

/// The number of definitions per change kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, JsonSchema)]
pub struct KindCounts {
    /// Definitions only the new side has.
    pub added: u64,
    /// Definitions only the old side has.
    pub removed: u64,
    /// Definitions both sides have that differ.
    pub changed: u64,
}

impl KindCounts {
    const fn add(&mut self, kind: SchemaChangeKind) {
        match kind {
            SchemaChangeKind::Added => self.added += 1,
            SchemaChangeKind::Removed => self.removed += 1,
            SchemaChangeKind::Changed => self.changed += 1,
        }
    }

    /// Returns the number of definitions that differ.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.added + self.removed + self.changed
    }
}

/// The number of changes per kind and level of the schema.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, JsonSchema)]
pub struct SchemaCounts {
    /// The struct changes.
    pub structs: KindCounts,
    /// The property changes of the changed structs.
    pub properties: KindCounts,
    /// The enum changes.
    pub enums: KindCounts,
    /// The option changes of the changed enums.
    pub options: KindCounts,
}

/// What a property is: its type, conversion and target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, JsonSchema)]
pub struct PropertyShape {
    /// The data type, e.g. `Reference`, `String` or `Class`; `Unknown(0x…)` for an unknown code.
    pub data_type: String,
    /// The conversion type: `0` for one value, any other code for an array of values.
    pub conversion: u16,
    /// The name of the struct or enum the values are of, point to or refer to; `null` for a
    /// property without a target.
    pub target: Option<String>,
}

impl fmt::Display for PropertyShape {
    /// Writes the data type, `[]` for an array and the target, e.g. `Reference Part` or
    /// `String[]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.data_type)?;
        if self.conversion != 0 {
            f.write_str("[]")?;
        }
        if let Some(target) = &self.target {
            write!(f, " {target}")?;
        }
        Ok(())
    }
}

/// An aspect of a property that differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PropertyAspect {
    /// The data type.
    DataType,
    /// The conversion type.
    Conversion,
    /// The target struct or enum.
    Target,
}

impl fmt::Display for PropertyAspect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DataType => "type",
            Self::Conversion => "conversion",
            Self::Target => "target",
        })
    }
}

/// One added, removed or changed property of a changed struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct PropertyChange {
    /// The property name.
    pub name: String,
    /// How the property differs.
    pub change: SchemaChangeKind,
    /// What differs, in the order data type, conversion, target; empty for an added or removed
    /// property.
    pub aspects: Vec<PropertyAspect>,
    /// The property on the old side, absent when it was added.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old: Option<PropertyShape>,
    /// The property on the new side, absent when it was removed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new: Option<PropertyShape>,
}

/// The parent of a struct on each side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ParentChange {
    /// The parent on the old side, `null` for none.
    pub old: Option<String>,
    /// The parent on the new side, `null` for none.
    pub new: Option<String>,
}

/// One added, removed or changed struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct StructChange {
    /// The struct name.
    pub name: String,
    /// How the struct differs.
    pub change: SchemaChangeKind,
    /// The Records of the struct and of the structs inheriting from it, on each side; `0` on the
    /// side that lacks the struct.
    pub records: SideCounts,
    /// The parents, present when a changed struct inherits from another struct than before.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<ParentChange>,
    /// The properties the struct declares itself that differ, ordered by name; empty for an added
    /// or removed struct.
    pub properties: Vec<PropertyChange>,
}

/// One added or removed option of a changed enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct OptionChange {
    /// The option name.
    pub name: String,
    /// `added` or `removed`.
    pub change: SchemaChangeKind,
}

/// One added, removed or changed enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct EnumChange {
    /// The enum name.
    pub name: String,
    /// How the enum differs.
    pub change: SchemaChangeKind,
    /// The Records of the structs that declare a property of this enum, and of the structs
    /// inheriting from them, on each side.
    pub records: SideCounts,
    /// The options added or removed, ordered by name; empty for an added or removed enum.
    pub options: Vec<OptionChange>,
}

/// The schema drift between two DataForges: the structs, properties, enums and enum options added,
/// removed and changed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, JsonSchema)]
pub struct SchemaDiff {
    /// The number of changes per kind and level.
    pub counts: SchemaCounts,
    /// Every added, removed or changed struct, ordered by name.
    pub structs: Vec<StructChange>,
    /// Every added, removed or changed enum, ordered by name.
    pub enums: Vec<EnumChange>,
    /// The definitions whose name cannot be read and that were left out, on each side.
    pub unreadable: SideCounts,
}

impl SchemaDiff {
    /// Returns whether the schemas differ.
    #[must_use]
    pub const fn has_differences(&self) -> bool {
        !self.structs.is_empty() || !self.enums.is_empty()
    }
}

/// Compares the schema of the `old` with the `new` DataForge: its structs with the properties
/// they declare, and its enums with their options, joined by name.
///
/// A property differs in its data type, conversion type or target; a struct also in its parent.
/// The Records of a struct are those of the struct and of the structs inheriting from it, so a
/// property change counts the Records that carry the property.
#[must_use]
pub fn compare_schema(old: &DataForge<'_>, new: &DataForge<'_>) -> SchemaDiff {
    let (old, new) = rayon::join(|| SideSchema::read(old), || SideSchema::read(new));
    let mut diff = SchemaDiff {
        unreadable: SideCounts {
            old: old.unreadable,
            new: new.unreadable,
        },
        ..SchemaDiff::default()
    };
    diff.structs = join(&old.structs, &new.structs, struct_change);
    diff.enums = join(&old.enums, &new.enums, enum_change);
    for change in &diff.structs {
        diff.counts.structs.add(change.change);
        for property in &change.properties {
            diff.counts.properties.add(property.change);
        }
    }
    for change in &diff.enums {
        diff.counts.enums.add(change.change);
        for option in &change.options {
            diff.counts.options.add(option.change);
        }
    }
    diff
}

fn join<'f, T, C>(
    old: &BTreeMap<&'f str, T>,
    new: &BTreeMap<&'f str, T>,
    change: impl Fn(&str, Option<&T>, Option<&T>) -> Option<C>,
) -> Vec<C> {
    let mut changes = Vec::new();
    let mut before = old.iter().peekable();
    let mut after = new.iter().peekable();
    loop {
        let next = match (before.peek(), after.peek()) {
            (None, None) => break,
            (Some(&(name, _)), None) => (*name, true, false),
            (None, Some(&(name, _))) => (*name, false, true),
            (Some(&(old_name, _)), Some(&(new_name, _))) => match old_name.cmp(new_name) {
                std::cmp::Ordering::Less => (*old_name, true, false),
                std::cmp::Ordering::Greater => (*new_name, false, true),
                std::cmp::Ordering::Equal => (*old_name, true, true),
            },
        };
        let (name, in_old, in_new) = next;
        let old_value = if in_old {
            before.next().map(|(_, value)| value)
        } else {
            None
        };
        let new_value = if in_new {
            after.next().map(|(_, value)| value)
        } else {
            None
        };
        changes.extend(change(name, old_value, new_value));
    }
    changes
}

fn struct_change(
    name: &str,
    old: Option<&StructDef<'_>>,
    new: Option<&StructDef<'_>>,
) -> Option<StructChange> {
    let records = SideCounts {
        old: old.map_or(0, |def| def.records),
        new: new.map_or(0, |def| def.records),
    };
    let (change, parent, properties) = match (old, new) {
        (Some(_), None) => (SchemaChangeKind::Removed, None, Vec::new()),
        (None, Some(_)) => (SchemaChangeKind::Added, None, Vec::new()),
        (Some(old), Some(new)) => {
            let properties = join(&old.properties, &new.properties, property_change);
            let parent = (old.parent != new.parent).then(|| ParentChange {
                old: old.parent.map(str::to_owned),
                new: new.parent.map(str::to_owned),
            });
            if parent.is_none() && properties.is_empty() {
                return None;
            }
            (SchemaChangeKind::Changed, parent, properties)
        }
        (None, None) => return None,
    };
    Some(StructChange {
        name: name.to_owned(),
        change,
        records,
        parent,
        properties,
    })
}

fn property_change(
    name: &str,
    old: Option<&PropertyShape>,
    new: Option<&PropertyShape>,
) -> Option<PropertyChange> {
    let (change, aspects) = match (old, new) {
        (Some(_), None) => (SchemaChangeKind::Removed, Vec::new()),
        (None, Some(_)) => (SchemaChangeKind::Added, Vec::new()),
        (Some(old), Some(new)) => {
            let aspects: Vec<PropertyAspect> = [
                (old.data_type != new.data_type, PropertyAspect::DataType),
                (old.conversion != new.conversion, PropertyAspect::Conversion),
                (old.target != new.target, PropertyAspect::Target),
            ]
            .into_iter()
            .filter_map(|(differs, aspect)| differs.then_some(aspect))
            .collect();
            if aspects.is_empty() {
                return None;
            }
            (SchemaChangeKind::Changed, aspects)
        }
        (None, None) => return None,
    };
    Some(PropertyChange {
        name: name.to_owned(),
        change,
        aspects,
        old: old.cloned(),
        new: new.cloned(),
    })
}

fn enum_change(
    name: &str,
    old: Option<&EnumDef<'_>>,
    new: Option<&EnumDef<'_>>,
) -> Option<EnumChange> {
    let records = SideCounts {
        old: old.map_or(0, |def| def.records),
        new: new.map_or(0, |def| def.records),
    };
    let (change, options) = match (old, new) {
        (Some(_), None) => (SchemaChangeKind::Removed, Vec::new()),
        (None, Some(_)) => (SchemaChangeKind::Added, Vec::new()),
        (Some(old), Some(new)) => {
            let removed = old
                .options
                .difference(&new.options)
                .map(|name| OptionChange {
                    name: (*name).to_owned(),
                    change: SchemaChangeKind::Removed,
                });
            let added = new
                .options
                .difference(&old.options)
                .map(|name| OptionChange {
                    name: (*name).to_owned(),
                    change: SchemaChangeKind::Added,
                });
            let mut options: Vec<OptionChange> = removed.chain(added).collect();
            if options.is_empty() {
                return None;
            }
            options.sort_by(|a, b| a.name.cmp(&b.name));
            (SchemaChangeKind::Changed, options)
        }
        (None, None) => return None,
    };
    Some(EnumChange {
        name: name.to_owned(),
        change,
        records,
        options,
    })
}
