use std::collections::{HashMap, HashSet};

use p4kx_dataforge::query::{Query, QueryErrorKind};
use p4kx_dataforge::{
    DataForge, JsonLayout, MAX_INHERITANCE_DEPTH, RecordJson, RecordTree, RecordXml, TreeBuilder,
    Value,
};
use rayon::prelude::*;

use crate::coverage::{
    ComponentCoverage, ReferenceItem, ReferenceKind, SampleExport, TypeCoverage,
};

const ENTITY_TYPE: &str = "EntityClassDefinition";
const COMPONENTS: &str = "Components";

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

#[derive(Debug, Clone, Copy, Default)]
struct ComponentTally {
    entities: u64,
    instances: u64,
}

/// The names the scan looks for and the parent of every struct by name.
struct Context<'a> {
    types: HashMap<&'a str, usize>,
    components: HashMap<&'a str, usize>,
    parents: HashMap<&'a str, &'a str>,
}

struct Pass<'f> {
    tallies: Vec<Tally>,
    components: Vec<ComponentTally>,
    entities: u64,
    unreadable_entities: u64,
    builder: TreeBuilder<'f>,
    matched: Vec<usize>,
    hits: Vec<usize>,
}

impl Pass<'_> {
    fn new(types: usize, components: usize) -> Self {
        Self {
            tallies: vec![Tally::default(); types],
            components: vec![ComponentTally::default(); components],
            entities: 0,
            unreadable_entities: 0,
            builder: TreeBuilder::new(),
            matched: Vec::new(),
            hits: Vec::new(),
        }
    }

    fn merge(mut self, other: Self) -> Self {
        for (tally, other) in self.tallies.iter_mut().zip(other.tallies) {
            tally.merge(other);
        }
        for (tally, other) in self.components.iter_mut().zip(other.components) {
            tally.entities += other.entities;
            tally.instances += other.instances;
        }
        self.entities += other.entities;
        self.unreadable_entities += other.unreadable_entities;
        self
    }
}

/// What the DataForge holds of the listed Record types and components.
pub(crate) struct ForgeCoverage {
    pub(crate) types: Vec<TypeCoverage>,
    pub(crate) components: Vec<ComponentCoverage>,
    pub(crate) unreadable_entities: u64,
}

/// Counts the Records of the listed types, reads them and exports a sample of each, and counts
/// the listed components, derived structs included, in the `EntityClassDefinition` Records.
pub(crate) fn check(forge: &DataForge<'_>, items: &[ReferenceItem<'_>]) -> ForgeCoverage {
    let defined: HashSet<&str> = forge
        .structs()
        .filter_map(|found| found.name().ok())
        .collect();
    let types = of_kind(items, |kind| {
        matches!(kind, ReferenceKind::Record | ReferenceKind::Candidate)
    });
    let components = of_kind(items, |kind| kind == ReferenceKind::Component);
    let context = Context {
        types: first_slots(&types),
        components: first_slots(&components),
        parents: forge
            .structs()
            .filter_map(|found| Some((found.name().ok()?, found.parent()?.name().ok()?)))
            .collect(),
    };
    let pass = scan(forge, &context, types.len(), components.len());
    ForgeCoverage {
        types: types
            .iter()
            .map(|item| {
                let tally = context
                    .types
                    .get(item.name)
                    .and_then(|first| pass.tallies.get(*first))
                    .copied()
                    .unwrap_or_default();
                type_coverage(forge, item, &defined, &tally)
            })
            .collect(),
        components: components
            .iter()
            .map(|item| {
                let tally = context
                    .components
                    .get(item.name)
                    .and_then(|first| pass.components.get(*first))
                    .copied()
                    .unwrap_or_default();
                ComponentCoverage {
                    area: item.area.to_owned(),
                    name: item.name.to_owned(),
                    defined: defined.contains(item.name),
                    entities: tally.entities,
                    instances: tally.instances,
                    reachable: reachable(forge, item.name),
                }
            })
            .collect(),
        unreadable_entities: pass.unreadable_entities,
    }
}

fn of_kind<'a>(
    items: &[ReferenceItem<'a>],
    wanted: impl Fn(ReferenceKind) -> bool,
) -> Vec<ReferenceItem<'a>> {
    items
        .iter()
        .filter(|item| wanted(item.kind))
        .copied()
        .collect()
}

fn first_slots<'a>(items: &[ReferenceItem<'a>]) -> HashMap<&'a str, usize> {
    let mut slots = HashMap::new();
    for (index, item) in items.iter().enumerate() {
        slots.entry(item.name).or_insert(index);
    }
    slots
}

fn scan<'f>(
    forge: &'f DataForge<'f>,
    context: &Context<'_>,
    types: usize,
    components: usize,
) -> Pass<'f> {
    let count = u32::try_from(forge.record_count()).unwrap_or(u32::MAX);
    (0..count)
        .into_par_iter()
        .fold(
            || Pass::new(types, components),
            |mut pass, index| {
                visit(forge, context, index, &mut pass);
                pass
            },
        )
        .reduce(|| Pass::new(types, components), Pass::merge)
}

fn visit<'f>(forge: &'f DataForge<'f>, context: &Context<'_>, index: u32, pass: &mut Pass<'f>) {
    let Some(record) = forge.record(index) else {
        return;
    };
    pass.matched.clear();
    let mut entity = false;
    let mut cyclic = false;
    let mut current = record.struct_type().ok();
    let mut steps = 0;
    while let Some(found) = current {
        if steps == MAX_INHERITANCE_DEPTH {
            cyclic = true;
            break;
        }
        steps += 1;
        if let Ok(name) = found.name() {
            entity |= name == ENTITY_TYPE;
            if let Some(slot) = context.types.get(name) {
                pass.matched.push(*slot);
            }
        }
        current = found.parent();
    }
    pass.matched.sort_unstable();
    pass.matched.dedup();
    let entity = entity && !context.components.is_empty();
    if pass.matched.is_empty() && !entity {
        return;
    }
    if entity {
        pass.entities += 1;
    }
    let read = if cyclic {
        None
    } else {
        pass.builder.build(&record).ok()
    };
    for slot in &pass.matched {
        let Some(tally) = pass.tallies.get_mut(*slot) else {
            continue;
        };
        tally.records += 1;
        match &read {
            Some(tree) => {
                tally.errors += tree.error_count() as u64;
                tally.sample = Some(tally.sample.map_or(index, |first| first.min(index)));
            }
            None => tally.unreadable += 1,
        }
    }
    if entity {
        match &read {
            Some(tree) => count_components(context, tree, pass),
            None => pass.unreadable_entities += 1,
        }
    }
}

fn count_components(context: &Context<'_>, tree: &RecordTree<'_>, pass: &mut Pass<'_>) {
    let Some(Value::Array(items)) = tree.field(COMPONENTS) else {
        return;
    };
    pass.hits.clear();
    for item in items {
        let Value::Struct(component) = item else {
            continue;
        };
        let mut name = Some(component.type_name());
        let mut steps = 0;
        while let Some(current) = name
            && steps < MAX_INHERITANCE_DEPTH
        {
            steps += 1;
            if let Some(slot) = context.components.get(current) {
                if let Some(tally) = pass.components.get_mut(*slot) {
                    tally.instances += 1;
                }
                pass.hits.push(*slot);
            }
            name = context.parents.get(current).copied();
        }
    }
    pass.hits.sort_unstable();
    pass.hits.dedup();
    for slot in &pass.hits {
        if let Some(tally) = pass.components.get_mut(*slot) {
            tally.entities += 1;
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

/// Returns whether the Component selector resolves: the Query compiles, or fails only because
/// the component is a struct and a path has to end at a scalar.
fn reachable(forge: &DataForge<'_>, component: &str) -> bool {
    let text = format!("from {ENTITY_TYPE} select {COMPONENTS}[{component}]");
    Query::parse(&text).is_ok_and(|query| match query.compile_with(forge, None) {
        Ok(_) => true,
        Err(error) => matches!(error.kind(), QueryErrorKind::UnsupportedField { .. }),
    })
}
