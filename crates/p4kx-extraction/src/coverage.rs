// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::str::FromStr;

use thiserror::Error;

const BUILTIN: &str = include_str!("../data/coverage-reference.txt");

/// What a line of the reference list names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReferenceKind {
    /// A Record type that is read completely.
    Record,
    /// A Record type named as a likely future need.
    Candidate,
    /// A component struct under `Components` of an `EntityClassDefinition`.
    Component,
    /// A folder of the Archive whose Entries are converted from CryXml.
    Entries,
    /// A `StarMapObjectType` name; the `StarMapObject` Records of that type are counted.
    StarMapType,
}

impl FromStr for ReferenceKind {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, ()> {
        match text {
            "record" => Ok(Self::Record),
            "candidate" => Ok(Self::Candidate),
            "component" => Ok(Self::Component),
            "entries" => Ok(Self::Entries),
            "starmap-type" => Ok(Self::StarMapType),
            _ => Err(()),
        }
    }
}

/// One line of the reference list: a topic area, what is named and its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceItem<'a> {
    /// The topic area, e.g. `missions`.
    pub area: &'a str,
    /// What the name is.
    pub kind: ReferenceKind,
    /// The type or component name, or for [`ReferenceKind::Entries`] the folder path.
    pub name: &'a str,
}

/// The game data a companion app reads, as a list of type names, component names and folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceList<'a> {
    items: Vec<ReferenceItem<'a>>,
}

/// A line of the reference list is not `<area> <kind> <name>`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("line {line} of the reference list is not `<area> <kind> <name>`")]
pub struct ReferenceError {
    line: usize,
}

impl ReferenceList<'static> {
    /// Returns the list built into P4KX.
    ///
    /// # Errors
    ///
    /// Returns a [`ReferenceError`] when the built-in list is malformed, which is a defect.
    pub fn builtin() -> Result<Self, ReferenceError> {
        Self::parse(BUILTIN)
    }
}

impl<'a> ReferenceList<'a> {
    /// Reads a list with one `<area> <kind> <name>` per line; blank lines are skipped.
    ///
    /// # Errors
    ///
    /// Returns a [`ReferenceError`] for a line of another shape or with an unknown kind.
    pub fn parse(text: &'a str) -> Result<Self, ReferenceError> {
        let mut items = Vec::new();
        for (index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let error = ReferenceError { line: index + 1 };
            let mut words = line.split_whitespace();
            let (Some(area), Some(kind), Some(name), None) =
                (words.next(), words.next(), words.next(), words.next())
            else {
                return Err(error);
            };
            let kind = kind.parse().map_err(|()| error.clone())?;
            items.push(ReferenceItem { area, kind, name });
        }
        Ok(Self { items })
    }

    /// Returns the items in list order.
    #[must_use]
    pub fn items(&self) -> &[ReferenceItem<'a>] {
        &self.items
    }
}

/// The coverage of one Record type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeCoverage {
    /// The topic area.
    pub area: String,
    /// The type name.
    pub name: String,
    /// Whether the type was named as a likely future need.
    pub candidate: bool,
    /// Whether the DataForge defines the type.
    pub defined: bool,
    /// The Records of the type and of the types deriving from it.
    pub records: u64,
    /// The Records whose layout or instance cannot be read.
    pub unreadable_records: u64,
    /// The values that cannot be read, over all Records of the type.
    pub error_values: u64,
    /// The export of the first readable Record, or why it failed; `None` without Records.
    pub sample: Option<SampleExport>,
}

/// The first readable Record of a type exported as Record XML and Record JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleExport {
    /// The record path of the Record.
    pub record_path: String,
    /// The size of its Record XML in bytes.
    pub xml_bytes: usize,
    /// The size of its Record JSON in bytes.
    pub json_bytes: usize,
    /// Why writing failed, if it did.
    pub error: Option<String>,
}

/// The coverage of one component struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentCoverage {
    /// The topic area.
    pub area: String,
    /// The component struct name.
    pub name: String,
    /// Whether the DataForge defines the struct.
    pub defined: bool,
    /// The `EntityClassDefinition` Records that hold at least one such component.
    pub entities: u64,
    /// The components of that type in all `EntityClassDefinition` Records.
    pub instances: u64,
    /// Whether the Component selector `Components[<name>]` of a Query over
    /// `EntityClassDefinition` resolves.
    pub reachable: bool,
}

/// The coverage of the `StarMapObject` Records of one `StarMapObjectType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectCoverage {
    /// The topic area.
    pub area: String,
    /// The `StarMapObjectType` name, e.g. `SolarSystem`.
    pub type_name: String,
    /// The `StarMapObject` Records, derived types included, whose type has that name.
    pub records: u64,
    /// Of `records`, those whose record path contains `template`, in any case.
    pub templates: u64,
    /// The export of the first such Record that is no template, or why it failed; `None` without
    /// one.
    pub sample: Option<SampleExport>,
}

/// The coverage of one Entry folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryCoverage {
    /// The topic area.
    pub area: String,
    /// The folder, as the reference list spells it.
    pub folder: String,
    /// The Entries in the folder.
    pub entries: u64,
    /// The Entries converted from CryXml to XML.
    pub converted: u64,
    /// The Entries that are text XML already.
    pub text: u64,
    /// The Entries that could not be read or converted, with the reason, in path order.
    pub failures: Vec<(String, String)>,
}

/// What the Archive holds of the reference list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoverageReport {
    /// The Record types, in list order.
    pub types: Vec<TypeCoverage>,
    /// The components, in list order.
    pub components: Vec<ComponentCoverage>,
    /// The `StarMapObject` Records per listed `StarMapObjectType` name, in list order.
    pub objects: Vec<ObjectCoverage>,
    /// The Entry folders, in list order.
    pub entries: Vec<EntryCoverage>,
    /// The `EntityClassDefinition` Records that cannot be built, when components are listed.
    pub unreadable_entities: u64,
}

impl CoverageReport {
    /// Returns whether everything listed exists, reads without error and converts.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unreadable_entities == 0
            && self.types.iter().all(|found| {
                found.candidate
                    || (found.records > 0
                        && found.unreadable_records == 0
                        && found.error_values == 0
                        && found
                            .sample
                            .as_ref()
                            .is_some_and(|sample| sample.error.is_none()))
            })
            && self
                .components
                .iter()
                .all(|found| found.instances > 0 && found.reachable)
            && self.objects.iter().all(|found| {
                found.records > 0
                    && found
                        .sample
                        .as_ref()
                        .is_some_and(|sample| sample.error.is_none())
            })
            && self
                .entries
                .iter()
                .all(|found| found.entries > 0 && found.failures.is_empty())
    }
}
