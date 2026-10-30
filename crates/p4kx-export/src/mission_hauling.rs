// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::StructValue;

use crate::contract_hauling::{
    HaulingItem, HaulingItemKind, HaulingOrder, HaulingRequirement, HaulingSource,
};
use crate::mission_index::{Broker, Template};
use crate::mission_layers::Layer;
use crate::record_fields::{integer, reference, structs, structure, text};

const ORDERS_PROPERTY: &str = "MissionPropertyValue_HaulingOrders";
const OR_CONTENT: &str = "HaulingOrderContent_Or";
const HAULING_HANDLER: &str = "ObjectiveHandler_Hauling";

/// A hauling orders property of one level: the variable it fills and what it asks for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct HaulingProperty<'f> {
    pub(crate) name: Option<&'f str>,
    pub(crate) requirements: Vec<HaulingRequirement>,
}

/// The hauling orders of the levels of one contract entry, nearest level first.
pub(crate) struct HaulingLevels<'x, 'f> {
    /// The property levels with the source each one stands for.
    pub(crate) properties: Vec<(HaulingSource, &'x [HaulingProperty<'f>])>,
    /// The orders of the hauling objectives of the template.
    pub(crate) template_objective: &'x [HaulingRequirement],
    /// The orders of the hauling objectives of the broker entry.
    pub(crate) broker_objective: &'x [HaulingRequirement],
}

impl<'x, 'f> HaulingLevels<'x, 'f> {
    /// Returns the levels of a contract entry: the layers of its sub contract (when it is one),
    /// contract and handler, its template and its broker entry.
    pub(crate) fn of(
        sub: Option<&'x Layer<'f>>,
        contract: &'x Layer<'f>,
        handler: &'x Layer<'f>,
        template: Option<&'x Template<'f>>,
        broker: Option<&'x Broker<'f>>,
    ) -> Self {
        let sub = sub.map(|layer| (HaulingSource::SubContract, layer.hauling.as_slice()));
        Self {
            properties: sub
                .into_iter()
                .chain([
                    (HaulingSource::Contract, contract.hauling.as_slice()),
                    (HaulingSource::Handler, handler.hauling.as_slice()),
                    (
                        HaulingSource::Template,
                        template.map_or(&[][..], |data| data.hauling.as_slice()),
                    ),
                ])
                .collect(),
            template_objective: template.map_or(&[][..], |data| data.objective_orders.as_slice()),
            broker_objective: broker.map_or(&[][..], |data| data.objective_orders.as_slice()),
        }
    }

    /// Returns the orders of the entry: per `missionVariableName` the property of the nearest
    /// level, then the objective orders of the template, else of the broker entry.
    pub(crate) fn orders(&self) -> Vec<HaulingOrder<'f>> {
        let mut seen: Vec<Option<&'f str>> = Vec::new();
        let mut orders = Vec::new();
        for (source, level) in &self.properties {
            for property in *level {
                if seen.contains(&property.name) {
                    continue;
                }
                seen.push(property.name);
                orders.push(HaulingOrder {
                    source: *source,
                    variable_name: property.name,
                    requirements: property.requirements.clone(),
                });
            }
        }
        let (source, objective) = if self.template_objective.is_empty() {
            (HaulingSource::BrokerObjective, self.broker_objective)
        } else {
            (HaulingSource::TemplateObjective, self.template_objective)
        };
        if !objective.is_empty() {
            orders.push(HaulingOrder {
                source,
                variable_name: None,
                requirements: objective.to_vec(),
            });
        }
        orders
    }
}

/// Reads the hauling orders properties the array field `name` of `parent` holds (the
/// `propertyOverrides` of a contract, the `contractProperties` of a template).
pub(crate) fn property_orders<'f>(
    parent: Option<&StructValue<'f>>,
    name: &str,
) -> Vec<HaulingProperty<'f>> {
    structs(parent, name)
        .filter_map(|property| {
            let value = structure(Some(property), "value")?;
            (value.type_name() == ORDERS_PROPERTY).then(|| HaulingProperty {
                name: text(Some(property), "missionVariableName"),
                requirements: structs(Some(value), "haulingOrderContent")
                    .filter_map(|content| requirement(content, false))
                    .collect(),
            })
        })
        .collect()
}

/// Reads the orders of the hauling objectives in the `objectiveTokens` of the Record `root`:
/// the orders that state a commodity or an entity themselves. Orders that only name a drop-off
/// or point at a property are left out; their content is the property's.
pub(crate) fn objective_orders(root: Option<&StructValue<'_>>) -> Vec<HaulingRequirement> {
    structs(root, "objectiveTokens")
        .filter_map(|token| structure(Some(token), "objectiveHandler"))
        .filter(|handler| handler.type_name() == HAULING_HANDLER)
        .flat_map(|handler| structs(Some(handler), "haulingOrders"))
        .filter_map(|order| requirement(order, true))
        .collect()
}

fn requirement(content: &StructValue<'_>, objective: bool) -> Option<HaulingRequirement> {
    if content.type_name() == OR_CONTENT {
        let options = structs(Some(content), "options")
            .map(|option| {
                structs(Some(option), "orders")
                    .filter_map(|order| item(order, objective))
                    .collect()
            })
            .collect();
        return Some(HaulingRequirement { options });
    }
    item(content, objective).map(|item| HaulingRequirement {
        options: vec![vec![item]],
    })
}

/// Reads one order. A mission item of an objective is a property reference, not a fact.
fn item(order: &StructValue<'_>, objective: bool) -> Option<HaulingItem> {
    let (_, suffix) = order.type_name().split_once('_')?;
    let kind = match suffix {
        "Resource" | "ResourceUnlimitedDropOff" => HaulingItemKind::Resource,
        "EntityClass" => HaulingItemKind::EntityClass,
        "EntityClasses" => HaulingItemKind::EntityClasses,
        "MissionItem" if !objective => HaulingItemKind::MissionItem,
        _ => return None,
    };
    let order = Some(order);
    let pieces = !matches!(kind, HaulingItemKind::Resource);
    Some(HaulingItem {
        kind,
        resource_guid: reference(order, "resource"),
        entity_class_guid: reference(order, "entityClass"),
        entity_classes_guid: reference(order, "haulingEntityClasses"),
        min_scu: integer(order, "minSCU"),
        max_scu: integer(order, "maxSCU"),
        max_container_size: integer(order, "maxContainerSize"),
        min_amount: integer(order, "minAmount").filter(|_| pieces),
        max_amount: integer(order, "maxAmount").filter(|_| pieces),
    })
}
