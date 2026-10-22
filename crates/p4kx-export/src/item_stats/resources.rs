use p4kx_extraction::{RecordTree, StructValue, Value};

use crate::contract_stats::{CoolerStats, PowerPlantStats, ResourceFlow, ResourceState, Signature};
use crate::record_fields::{component, elements, number, structure, text};

const GENERATION: &str = "generation";
const CONSUMPTION: &str = "consumption";

fn signature(parent: Option<&StructValue<'_>>, name: &str) -> Option<Signature> {
    let signature = structure(parent, name)?;
    Some(Signature {
        nominal: number(Some(signature), "nominalSignature"),
        decay_rate: number(Some(signature), "decayRate"),
    })
}

fn flow<'f>(
    delta: &StructValue<'f>,
    direction: &'static str,
) -> Option<ResourceFlow<'f>> {
    let side = structure(Some(delta), direction)?;
    let unit = structure(Some(side), "resourceAmountPerSecond");
    let amount = unit.and_then(|unit| {
        unit.fields()
            .iter()
            .find_map(|field| number(Some(unit), field.name()))
    });
    Some(ResourceFlow {
        kind: delta.type_name(),
        direction,
        resource: text(Some(side), "resource"),
        amount,
        unit: unit.map(StructValue::type_name),
    })
}

/// Returns the states of the resource component of `tree`: the name, the EM and IR signature
/// and the generation and consumption flows of each.
pub(super) fn states<'f>(tree: &RecordTree<'f>) -> Vec<ResourceState<'f>> {
    let resources = component(tree, "ItemResourceComponentParams");
    elements(resources, "states")
        .iter()
        .filter_map(|state| match state {
            Value::Struct(state) => Some(state),
            _ => None,
        })
        .map(|state| {
            let signatures = structure(Some(state), "signatureParams");
            let flows = elements(Some(state), "deltas")
                .iter()
                .filter_map(|delta| match delta {
                    Value::Struct(delta) => Some(delta),
                    _ => None,
                })
                .flat_map(|delta| {
                    [GENERATION, CONSUMPTION]
                        .into_iter()
                        .filter_map(move |direction| flow(delta, direction))
                })
                .collect();
            ResourceState {
                name: text(Some(state), "name"),
                em_signature: signature(signatures, "EMSignature"),
                ir_signature: signature(signatures, "IRSignature"),
                flows,
            }
        })
        .collect()
}

fn generated(states: &[ResourceState<'_>], resource: &str) -> Option<f32> {
    states
        .iter()
        .flat_map(|state| &state.flows)
        .filter(|flow| flow.direction == GENERATION)
        .find(|flow| {
            flow.resource
                .is_some_and(|name| name.eq_ignore_ascii_case(resource))
        })
        .and_then(|flow| flow.amount)
}

pub(super) fn power_plant(states: &[ResourceState<'_>]) -> PowerPlantStats {
    PowerPlantStats {
        power_output: generated(states, "Power"),
    }
}

pub(super) fn cooler(states: &[ResourceState<'_>]) -> CoolerStats {
    CoolerStats {
        cooling_rate: generated(states, "Coolant"),
    }
}
