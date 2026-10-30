// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::contract_hauling::{HaulingOrder, HaulingRequirement, HaulingSource};
use crate::mission_facts::Levels;
use crate::mission_hauling::HaulingProperty;
use crate::mission_index::{Broker, Template};

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
        levels: &Levels<'x, 'f>,
        template: Option<&'x Template<'f>>,
        broker: Option<&'x Broker<'f>>,
    ) -> Self {
        let sub = levels
            .sub
            .map(|layer| (HaulingSource::SubContract, layer.hauling.as_slice()));
        Self {
            properties: sub
                .into_iter()
                .chain([
                    (HaulingSource::Contract, levels.contract.hauling.as_slice()),
                    (HaulingSource::Handler, levels.handler.hauling.as_slice()),
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
    /// level (a later level only fills a variable an earlier one leaves open), then the objective
    /// orders of the template, else of the broker entry. A property without a name fills no
    /// variable and shadows nothing, so every one of them is listed.
    pub(crate) fn orders(&self) -> Vec<HaulingOrder<'f>> {
        let mut seen: Vec<&'f str> = Vec::new();
        let mut orders = Vec::new();
        for (source, level) in &self.properties {
            for property in *level {
                if let Some(name) = property.name {
                    if seen.contains(&name) {
                        continue;
                    }
                    seen.push(name);
                }
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
