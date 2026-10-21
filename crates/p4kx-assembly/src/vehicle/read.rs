use std::collections::HashMap;

use super::{PartKind, PortVisibility, VehicleItemPort, VehiclePart};
use crate::markup::Element;
use crate::merge::port_key;

/// Reads the parts below the `Parts` element of a `Vehicle` element; parts marked `skipPart`
/// are left out with everything below them.
pub(super) fn parts(vehicle: &Element) -> Vec<VehiclePart> {
    vehicle
        .children_named("Parts")
        .flat_map(part_list)
        .collect()
}

fn part_list(container: &Element) -> Vec<VehiclePart> {
    container
        .children
        .iter()
        .flat_map(|child| {
            if child.name.eq_ignore_ascii_case("Part") {
                part(child).into_iter().collect()
            } else if child.name.eq_ignore_ascii_case("Parts") {
                part_list(child)
            } else {
                Vec::new()
            }
        })
        .collect()
}

fn part(element: &Element) -> Option<VehiclePart> {
    if is_set(element.attribute("skipPart")) {
        return None;
    }
    let class = element.attribute("class").unwrap_or_default();
    let kind = kind_of(class);
    Some(VehiclePart {
        name: element.attribute("name").unwrap_or_default().to_owned(),
        class: class.to_owned(),
        kind,
        mass: number(element.attribute("mass")),
        damage_max: number(element.attribute("damageMax")),
        detach_ratio: number(element.attribute("detachRatio")),
        attributes: element.attributes.clone(),
        item_port: element.child("ItemPort").map(item_port),
        tread_wheels: if kind == PartKind::Tread {
            tread_wheels(element)
        } else {
            Vec::new()
        },
        children: part_list(element),
    })
}

const fn kind_of(class: &str) -> PartKind {
    if class.eq_ignore_ascii_case("ItemPort") {
        PartKind::ItemPort
    } else if class.eq_ignore_ascii_case("Tread") {
        PartKind::Tread
    } else if class.eq_ignore_ascii_case("SubPartWheel") {
        PartKind::Wheel
    } else {
        PartKind::Other
    }
}

fn item_port(element: &Element) -> VehicleItemPort {
    VehicleItemPort {
        min_size: size(element.attribute("minSize")),
        max_size: size(element.attribute("maxSize")),
        flags: element.attribute("flags").unwrap_or_default().to_owned(),
        id: element.attribute("id").map(str::to_owned),
    }
}

fn tread_wheels(part: &Element) -> Vec<String> {
    part.child("Tread")
        .and_then(|tread| tread.child("Wheels"))
        .into_iter()
        .flat_map(|wheels| wheels.children_named("Wheel"))
        .filter_map(|wheel| wheel.attribute("partName"))
        .map(str::to_owned)
        .collect()
}

/// Returns whether an attribute holds a true flag: `1` or `true`.
fn is_set(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        let value = value.trim();
        value == "1" || value.eq_ignore_ascii_case("true")
    })
}

fn size(value: Option<&str>) -> Option<u32> {
    value?.trim().parse().ok()
}

/// Reads the longest numeric prefix of a value after trimming, e.g. `12.5` of `12.5kg`; a value
/// that does not start with a number, such as `v2 1500`, has none.
fn number(value: Option<&str>) -> Option<f64> {
    let text = value?.trim();
    let bytes = text.as_bytes();
    let digits = |from: usize| {
        bytes.get(from..).map_or(0, |rest| {
            rest.iter().take_while(|b| b.is_ascii_digit()).count()
        })
    };
    let mut end = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let whole = digits(end);
    end = end.saturating_add(whole);
    let mut fraction = 0;
    if bytes.get(end) == Some(&b'.') {
        fraction = digits(end.saturating_add(1));
        if whole > 0 || fraction > 0 {
            end = end.saturating_add(1).saturating_add(fraction);
        }
    }
    if whole == 0 && fraction == 0 {
        return None;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(
            bytes.get(end.saturating_add(1)),
            Some(b'+' | b'-')
        ));
        let start = end.saturating_add(1).saturating_add(sign);
        let exponent = digits(start);
        if exponent > 0 {
            end = start.saturating_add(exponent);
        }
    }
    let parsed = text.get(..end)?.parse::<f64>().ok()?;
    parsed.is_finite().then_some(parsed)
}
/// Returns the visibility of each item port by lower-case part name; the first part decides.
pub(super) fn visibility(parts: &[VehiclePart]) -> HashMap<String, PortVisibility> {
    let mut map = HashMap::new();
    collect_visibility(parts, &mut map);
    map
}

fn collect_visibility(parts: &[VehiclePart], map: &mut HashMap<String, PortVisibility>) {
    for part in parts {
        if let Some(port) = &part.item_port {
            map.entry(port_key(&part.name))
                .or_insert_with(|| port.visibility());
        }
        collect_visibility(&part.children, map);
    }
}
