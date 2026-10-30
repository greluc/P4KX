// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::StructValue;

use crate::contract_facts::{ContractFacts, FactLayer, RequiredIntro, Stated};
use crate::contract_missions::StandingLevel;
use crate::mission_index::Template;
use crate::mission_intros::required;
use crate::mission_layers::{Layer, one};
use crate::mission_results::RunTerms;
use crate::record_fields::{boolean, integer};

/// The `ContractAvailability` fields of a handler's `defaultAvailability`, as stored.
#[derive(Debug, Clone, Copy, Default)]
struct Availability {
    once_only: Option<bool>,
    max_players_per_instance: Option<i64>,
    available_in_prison: Option<bool>,
    can_reaccept_after_abandoning: Option<bool>,
    can_reaccept_after_failing: Option<bool>,
    hide_in_mobi_glas: Option<bool>,
}

/// The layers of one entry, nearest first: sub contract (when the entry is one), contract, handler.
pub(crate) struct Levels<'a, 'f> {
    pub(crate) sub: Option<&'a Layer<'f>>,
    pub(crate) contract: &'a Layer<'f>,
    pub(crate) handler: &'a Layer<'f>,
}

impl<'f> Levels<'_, 'f> {
    /// Returns the layers, nearest first.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Layer<'f>> {
        self.labelled().map(|(_, layer)| layer)
    }

    fn labelled(&self) -> impl Iterator<Item = (FactLayer, &Layer<'f>)> {
        self.sub
            .map(|layer| (FactLayer::SubContract, layer))
            .into_iter()
            .chain([
                (FactLayer::Contract, self.contract),
                (FactLayer::Handler, self.handler),
            ])
    }
}

/// What the entries of one contract share for their facts: its run terms, the availability of its
/// handler and the completion tags its handler and itself require.
#[derive(Debug, Default)]
pub(crate) struct SharedFacts {
    terms: RunTerms,
    availability: Availability,
    inherited_intros: Vec<RequiredIntro>,
}

impl SharedFacts {
    /// Reads the shared facts of `contract`, a contract of `handler`.
    pub(crate) fn read(handler: &StructValue<'_>, contract: &StructValue<'_>) -> Self {
        let availability = one(Some(handler), "defaultAvailability");
        Self {
            terms: RunTerms::read(contract),
            availability: Availability {
                once_only: boolean(availability, "onceOnly"),
                max_players_per_instance: integer(availability, "maxPlayersPerInstance"),
                available_in_prison: boolean(availability, "availableInPrison"),
                can_reaccept_after_abandoning: boolean(availability, "canReacceptAfterAbandoning"),
                can_reaccept_after_failing: boolean(availability, "canReacceptAfterFailing"),
                hide_in_mobi_glas: boolean(availability, "hideInMobiGlas"),
            },
            inherited_intros: required(StandingLevel::Handler, availability, "prerequisites")
                .into_iter()
                .chain(required(
                    StandingLevel::Contract,
                    Some(contract),
                    "additionalPrerequisites",
                ))
                .collect(),
        }
    }

    /// Returns the facts of the entry for `sub` (`None` for the contract itself) with the layers
    /// `levels` and `template`.
    pub(crate) fn facts(
        &self,
        sub: Option<&StructValue<'_>>,
        levels: &Levels<'_, '_>,
        template: Option<&Template<'_>>,
    ) -> ContractFacts {
        let availability = &self.availability;
        let flag = |param: &str, base: Option<Stated<bool>>| {
            stated(levels, |layer| layer.bool_param(param), base)
        };
        let handler = |value: Option<bool>| value.map(|value| at(value, FactLayer::Handler));
        let from_template = |value: Option<bool>| value.map(|value| at(value, FactLayer::Template));
        ContractFacts {
            time_to_complete: self.terms.time_to_complete,
            buy_in_amount: self.terms.buy_in_amount,
            can_be_shared: flag(
                "CanBeShared",
                from_template(template.and_then(|t| t.can_be_shared)),
            ),
            illegal: flag("Illegal", from_template(template.and_then(|t| t.illegal))),
            once_only: flag("OnceOnly", handler(availability.once_only)),
            max_players_per_instance: stated(
                levels,
                |layer| layer.int_param("MaxPlayersPerInstance"),
                availability
                    .max_players_per_instance
                    .map(|value| at(value, FactLayer::Handler)),
            ),
            available_in_prison: flag(
                "AvailableInPrison",
                handler(availability.available_in_prison),
            ),
            can_reaccept_after_abandoning: flag(
                "CanReacceptAfterAbandoning",
                handler(availability.can_reaccept_after_abandoning),
            ),
            can_reaccept_after_failing: flag(
                "CanReacceptAfterFailing",
                handler(availability.can_reaccept_after_failing),
            ),
            hide_in_mobi_glas: flag("HideInMobiGlas", handler(availability.hide_in_mobi_glas)),
            required_intros: self
                .inherited_intros
                .iter()
                .cloned()
                .chain(required(
                    StandingLevel::SubContract,
                    sub,
                    "additionalPrerequisites",
                ))
                .collect(),
        }
    }
}

const fn at<T>(value: T, layer: FactLayer) -> Stated<T> {
    Stated { value, layer }
}

/// Returns the value of the nearest layer in `levels` that `read` finds, else `base`.
fn stated<T>(
    levels: &Levels<'_, '_>,
    read: impl Fn(&Layer<'_>) -> Option<T>,
    base: Option<Stated<T>>,
) -> Option<Stated<T>> {
    levels
        .labelled()
        .find_map(|(layer, level)| read(level).map(|value| at(value, layer)))
        .or(base)
}
