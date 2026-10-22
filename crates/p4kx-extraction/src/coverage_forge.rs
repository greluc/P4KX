use std::collections::{HashMap, HashSet};

use p4kx_dataforge::query::{Query, QueryErrorKind};
use p4kx_dataforge::{
    DataForge, FieldStatistics, FieldStatisticsError, JsonLayout, RecordJson, RecordXml,
    TreeBuilder,
};
use rayon::prelude::*;
use thiserror::Error;

use crate::coverage::{
    ComponentCoverage, ReferenceItem, ReferenceKind, SampleExport, TypeCoverage,
};

const ENTITY_TYPE: &str = "EntityClassDefinition";
const COMPONENTS: &str = "Components";

/// The coverage cannot be computed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CoverageError {
    /// The `EntityClassDefinition` Records have too many distinct field paths to count.
    #[error("cannot count the components of the entities: {0}")]
    Components(#[from] FieldStatisticsError),
}

#[derive(Debug, Clone, Copy, Default)]
struct Tally {
    records: u64,
    unreadable: u64,
    errors: u64,
    sample: Option<u32>,
}

impl Tally {
    fn merge(&mut self, other: Self) {
        self.records += other.records;
        self.unreadable += other.unreadable;
        self.errors += other.errors;
        self.sample = match (self.sample, other.sample) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (left, right) => left.or(right),
        };
    }
}

struct Pass<'f> {
    tallies: Vec<Tally>,
    entities: Vec<u32>,
    builder: TreeBuilder<'f>,
    matched: Vec<usize>,
}

impl Pass<'_> {
    fn new(types: usize) -> Self {
        Self {
            tallies: vec![Tally::default(); types],
            entities: Vec::new(),
            builder: TreeBuilder::new(),
            matched: Vec::new(),
        }
    }

    fn merge(mut self, other: Self) -> Self {
        for (tally, other) in self.tallies.iter_mut().zip(other.tallies) {
            tally.merge(other);
        }
        self.entities.extend(other.entities);
        self
    }
}

/// Counts the Records of the listed types, reads them and exports a sample of each, then counts
/// the listed components in the `EntityClassDefinition` Records and checks that a Query reaches
/// them.
pub(crate) fn check(
    forge: &DataForge<'_>,
    items: &[ReferenceItem<'_>],
) -> Result<(Vec<TypeCoverage>, Vec<ComponentCoverage>), CoverageError> {
    let defined: HashSet<&str> = forge
        .structs()
        .filter_map(|found| found.name().ok())
        .collect();
    let types: Vec<&ReferenceItem<'_>> = items
        .iter()
        .filter(|item| matches!(item.kind, ReferenceKind::Record | ReferenceKind::Candidate))
        .collect();
    let wanted: HashMap<&str, usize> =
        types
            .iter()
            .enumerate()
            .fold(HashMap::new(), |mut wanted, (index, item)| {
                wanted.entry(item.name).or_insert(index);
                wanted
            });
    let mut pass = scan(forge, &wanted, types.len());
    pass.entities.sort_unstable();
    let records = types
        .iter()
        .map(|item| {
            let tally = wanted
                .get(item.name)
                .and_then(|first| pass.tallies.get(*first))
                .copied()
                .unwrap_or_default();
            type_coverage(forge, item, &defined, &tally)
        })
        .collect();
    let components = components(forge, items, &defined, &pass.entities)?;
    Ok((records, components))
}

fn scan<'f>(forge: &'f DataForge<'f>, wanted: &HashMap<&str, usize>, types: usize) -> Pass<'f> {
    let count = u32::try_from(forge.record_count()).unwrap_or(u32::MAX);
    (0..count)
        .into_par_iter()
        .fold(
            || Pass::new(types),
            |mut pass, index| {
                visit(forge, wanted, index, &mut pass);
                pass
            },
        )
        .reduce(|| Pass::new(types), Pass::merge)
}

fn visit<'f>(
    forge: &'f DataForge<'f>,
    wanted: &HashMap<&str, usize>,
    index: u32,
    pass: &mut Pass<'f>,
) {
    let Some(record) = forge.record(index) else {
        return;
    };
    let mut current = record.struct_type().ok();
    pass.matched.clear();
    let mut entity = false;
    while let Some(found) = current {
        if let Ok(name) = found.name() {
            entity |= name == ENTITY_TYPE;
            if let Some(slot) = wanted.get(name) {
                pass.matched.push(*slot);
            }
        }
        current = found.parent();
    }
    if entity {
        pass.entities.push(index);
    }
    if pass.matched.is_empty() {
        return;
    }
    let read = pass.builder.build(&record);
    for slot in &pass.matched {
        let Some(tally) = pass.tallies.get_mut(*slot) else {
            continue;
        };
        tally.records += 1;
        match &read {
            Ok(tree) => {
                tally.errors += tree.error_count() as u64;
                tally.sample = Some(tally.sample.map_or(index, |first| first.min(index)));
            }
            Err(_) => tally.unreadable += 1,
        }
    }
}

fn type_coverage(
    forge: &DataForge<'_>,
    item: &ReferenceItem<'_>,
    defined: &HashSet<&str>,
    tally: &Tally,
) -> TypeCoverage {
    TypeCoverage {
        area: item.area.to_owned(),
        name: item.name.to_owned(),
        candidate: item.kind == ReferenceKind::Candidate,
        defined: defined.contains(item.name),
        records: tally.records,
        unreadable_records: tally.unreadable,
        error_values: tally.errors,
        sample: tally.sample.map(|index| sample(forge, index)),
    }
}

fn sample(forge: &DataForge<'_>, index: u32) -> SampleExport {
    let Some(record) = forge.record(index) else {
        return failed(String::new(), "the Record is gone".to_owned());
    };
    let path = record.record_path().unwrap_or_default().to_owned();
    let tree = match record.tree() {
        Ok(tree) => tree,
        Err(error) => return failed(path, error.to_string()),
    };
    let mut xml = Vec::new();
    let mut json = Vec::new();
    let written = RecordXml::new()
        .write(&tree, &mut xml)
        .and_then(|()| RecordJson::new(JsonLayout::Pretty).write(&tree, &mut json));
    SampleExport {
        record_path: path,
        xml_bytes: xml.len(),
        json_bytes: json.len(),
        error: written.err().map(|error| error.to_string()),
    }
}

const fn failed(record_path: String, error: String) -> SampleExport {
    SampleExport {
        record_path,
        xml_bytes: 0,
        json_bytes: 0,
        error: Some(error),
    }
}

fn components(
    forge: &DataForge<'_>,
    items: &[ReferenceItem<'_>],
    defined: &HashSet<&str>,
    entities: &[u32],
) -> Result<Vec<ComponentCoverage>, CoverageError> {
    let listed: Vec<&ReferenceItem<'_>> = items
        .iter()
        .filter(|item| item.kind == ReferenceKind::Component)
        .collect();
    if listed.is_empty() {
        return Ok(Vec::new());
    }
    let statistics = FieldStatistics::of_records(forge, entities)?;
    Ok(listed
        .into_iter()
        .map(|item| {
            let found = statistics.get(&format!("{COMPONENTS}/{}", item.name));
            ComponentCoverage {
                area: item.area.to_owned(),
                name: item.name.to_owned(),
                defined: defined.contains(item.name),
                entities: found.map_or(0, p4kx_dataforge::FieldStat::records),
                instances: found.map_or(0, p4kx_dataforge::FieldStat::occurrences),
                reachable: reachable(forge, item.name),
            }
        })
        .collect())
}

/// Returns whether the Component selector resolves: the Query compiles, or fails only because
/// the component is a struct and a path has to end at a scalar.
fn reachable(forge: &DataForge<'_>, component: &str) -> bool {
    let text = format!("from {ENTITY_TYPE} select {COMPONENTS}[{component}]");
    Query::parse(&text).is_ok_and(|query| match query.compile_with(forge, None) {
        Ok(_) => true,
        Err(error) => matches!(error.kind(), QueryErrorKind::UnsupportedField { .. }),
    })
}
