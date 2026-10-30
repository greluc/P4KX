// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};

use p4kx_extraction::{Guid, StructValue};

use crate::contract::LocalisedText;
use crate::contract_missions::PickupPlace;
use crate::contract_starmap::StarmapObject;
use crate::guid_text::CatalogGuid;
use crate::mission_index::{Broker, MissionIndex};
use crate::mission_layers::items;
use crate::record_fields::reference;

/// The most pickup places an entry lists.
pub(crate) const MAX_PICKUP_PLACES: usize = 5;

const LOCALITY: &str = "ContractPrerequisite_Locality";
const LOCATION: &str = "ContractPrerequisite_Location";
const SYSTEM: &str = "SolarSystem";
const STAR: &str = "Star";

/// The places a level of a contract names in its prerequisites: the `MissionLocality` Records of
/// `ContractPrerequisite_Locality` and the starmap objects of `ContractPrerequisite_Location`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Where {
    pub(crate) localities: Vec<Guid>,
    pub(crate) locations: Vec<Guid>,
}

impl Where {
    /// Reads the prerequisites in the array field `name` of `parent`.
    pub(crate) fn read(parent: Option<&StructValue<'_>>, name: &str) -> Self {
        let mut found = Self::default();
        for prerequisite in items(parent, name)
            .iter()
            .filter_map(crate::blueprint::as_struct)
        {
            match prerequisite.type_name() {
                LOCALITY => found
                    .localities
                    .extend(reference(Some(prerequisite), "localityAvailable").map(|guid| guid.0)),
                LOCATION => found
                    .locations
                    .extend(reference(Some(prerequisite), "locationAvailable").map(|guid| guid.0)),
                _ => {}
            }
        }
        found
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.localities.is_empty() && self.locations.is_empty()
    }
}

/// The starmap objects by GUID, to turn the places of a contract into names.
pub(crate) struct Places<'s, 'f> {
    objects: HashMap<Guid, &'s StarmapObject<'f>>,
}

impl<'s, 'f> Places<'s, 'f> {
    pub(crate) fn new(objects: &'s [StarmapObject<'f>]) -> Self {
        Self {
            objects: objects
                .iter()
                .map(|object| (object.guid.0, object))
                .collect(),
        }
    }

    /// Returns the pickup places of the levels `levels`, nearest first: the starmap objects of
    /// their localities and locations, a star left out, the system first, then planets and moons,
    /// then the rest, each group in stored order, at most [`MAX_PICKUP_PLACES`], none twice.
    pub(crate) fn pickup(
        &self,
        index: &MissionIndex<'f>,
        levels: &[&Where],
    ) -> Vec<PickupPlace<'f>> {
        let mut seen: HashSet<Guid> = HashSet::new();
        let mut ranked: Vec<(u8, Guid, &StarmapObject<'f>)> = Vec::new();
        let listed = levels.iter().flat_map(|level| {
            level
                .localities
                .iter()
                .flat_map(|locality| index.locality(*locality).iter().copied())
                .chain(level.locations.iter().copied())
        });
        for guid in listed {
            if !seen.insert(guid) {
                continue;
            }
            let Some(object) = self.objects.get(&guid).copied() else {
                continue;
            };
            let kind = object.r#type;
            if kind.is_some_and(|kind| kind.eq_ignore_ascii_case(STAR)) {
                continue;
            }
            let rank = match kind {
                Some(kind) if kind.eq_ignore_ascii_case(SYSTEM) => 0,
                Some(kind)
                    if kind.eq_ignore_ascii_case("Planet") || kind.eq_ignore_ascii_case("Moon") =>
                {
                    1
                }
                _ => 2,
            };
            ranked.push((rank, guid, object));
        }
        ranked.sort_by_key(|(rank, _, _)| *rank);
        ranked
            .into_iter()
            .take(MAX_PICKUP_PLACES)
            .map(|(_, guid, object)| PickupPlace {
                starmap_object_guid: CatalogGuid(guid),
                object_type: object.r#type,
                place: LocalisedText {
                    key: object.name.key,
                    en: object.name.en.clone(),
                    de: object.name.de.clone(),
                },
            })
            .collect()
    }

    /// Returns the pickup places of a contract, or of its sub contract `sub`: those of the sub
    /// contract's own prerequisites, else of `inherited` (contract and handler), else the
    /// location of the broker entry.
    pub(crate) fn of_contract(
        &self,
        index: &MissionIndex<'f>,
        sub: Option<&StructValue<'_>>,
        (inherited, broker): (&[&Where], Option<&Broker<'f>>),
    ) -> Vec<PickupPlace<'f>> {
        let own = sub.map(|sub| Where::read(Some(sub), "additionalPrerequisites"));
        let wheres: Vec<&Where> = own
            .as_ref()
            .filter(|own| !own.is_empty())
            .map_or_else(|| inherited.to_vec(), |own| vec![own]);
        let pickup = self.pickup(index, &wheres);
        match broker.and_then(|broker| broker.location) {
            Some(location) if pickup.is_empty() => {
                let at_broker = Where {
                    localities: Vec::new(),
                    locations: vec![location],
                };
                self.pickup(index, &[&at_broker])
            }
            _ => pickup,
        }
    }
}
