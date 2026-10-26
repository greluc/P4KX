// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use crate::diff::{Changes, Trail, hex, text};
use crate::{ItemPort, Material, NameTable, SubMaterial};

/// How the items of a list are paired, named and shown when only one file has them.
struct Kind<T> {
    key: fn(&T) -> u32,
    name: for<'n> fn(&'n NameTable, u32) -> Option<&'n str>,
    value: fn(&T) -> String,
}

type Pair<'a, T> = (u32, usize, Option<&'a T>, Option<&'a T>);

fn ordinals<T>(items: &[T], key: fn(&T) -> u32) -> Vec<((u32, usize), &T)> {
    let mut seen: HashMap<u32, usize> = HashMap::with_capacity(items.len());
    items
        .iter()
        .map(|item| {
            let hash = key(item);
            let ordinal = seen.entry(hash).or_default();
            let at = *ordinal;
            *ordinal += 1;
            ((hash, at), item)
        })
        .collect()
}

/// Pairs the n-th item with a hash in one list with the n-th in the other. The pairs follow the
/// first list, the items only in the second come last.
fn pair<'a, T>(old: &'a [T], new: &'a [T], key: fn(&T) -> u32) -> Vec<Pair<'a, T>> {
    let new_items = ordinals(new, key);
    let mut unmatched: HashMap<(u32, usize), &T> = new_items.iter().copied().collect();
    let mut pairs: Vec<_> = ordinals(old, key)
        .into_iter()
        .map(|((hash, ordinal), item)| {
            (
                hash,
                ordinal,
                Some(item),
                unmatched.remove(&(hash, ordinal)),
            )
        })
        .collect();
    pairs.extend(
        new_items
            .into_iter()
            .filter(|(key, _)| unmatched.contains_key(key))
            .map(|((hash, ordinal), item)| (hash, ordinal, None, Some(item))),
    );
    pairs
}

fn each<T>(
    changes: &mut Changes<'_>,
    trail: &Trail,
    lists: (&[T], &[T]),
    kind: &Kind<T>,
    both: impl Fn(&mut Changes<'_>, &Trail, &T, &T),
) {
    for (hash, ordinal, old, new) in pair(lists.0, lists.1, kind.key) {
        let at = trail.hashed(hash, ordinal, (kind.name)(changes.names, hash));
        match (old, new) {
            (Some(old), Some(new)) => both(changes, &at, old, new),
            (Some(old), None) => changes.removed(&at, (kind.value)(old)),
            (None, Some(new)) => changes.added(&at, (kind.value)(new)),
            (None, None) => {}
        }
    }
}

fn port_text(port: &ItemPort) -> String {
    format!("item {}", port.item)
}

pub(crate) fn diff_ports(
    changes: &mut Changes<'_>,
    trail: &Trail,
    old: Option<&ItemPort>,
    new: Option<&ItemPort>,
) {
    match (old, new) {
        (Some(old), Some(new)) => {
            changes.field(trail, "port", &old.port, &new.port, hex);
            port_fields(changes, trail, old, new);
        }
        (Some(old), None) => changes.removed(trail, port_text(old)),
        (None, Some(new)) => changes.added(trail, port_text(new)),
        (None, None) => {}
    }
}

fn port_fields(changes: &mut Changes<'_>, trail: &Trail, old: &ItemPort, new: &ItemPort) {
    changes.field(trail, "item", &old.item, &new.item, text);
    changes.field(trail, "tail_count", &old.tail_count, &new.tail_count, text);
    let kind = Kind {
        key: |port: &ItemPort| port.port,
        name: NameTable::port,
        value: port_text,
    };
    each(
        changes,
        trail,
        (&old.children, &new.children),
        &kind,
        port_fields,
    );
}

pub(crate) fn diff_materials(
    changes: &mut Changes<'_>,
    trail: &Trail,
    old: &[Material],
    new: &[Material],
) {
    let kind = Kind {
        key: |material: &Material| material.attachment,
        name: NameTable::attachment,
        value: |material| format!("base material {}", material.base_material),
    };
    each(changes, trail, (old, new), &kind, material);
}

fn material(changes: &mut Changes<'_>, at: &Trail, old: &Material, new: &Material) {
    changes.field(
        at,
        "base_material",
        &old.base_material,
        &new.base_material,
        text,
    );
    changes.field(at, "flags", &old.flags, &new.flags, |flags| {
        format!("{flags:#010x}")
    });
    let kind = Kind {
        key: |sub: &SubMaterial| sub.name,
        name: NameTable::sub_material,
        value: |_| "present".to_owned(),
    };
    each(
        changes,
        &at.key("sub_materials"),
        (&old.sub_materials, &new.sub_materials),
        &kind,
        sub_material,
    );
}

fn sub_material(changes: &mut Changes<'_>, at: &Trail, old: &SubMaterial, new: &SubMaterial) {
    let textures = at.key("textures");
    for slot in sorted_slots(old, new) {
        let find = |sub: &SubMaterial| sub.textures.iter().find(|t| t.slot == slot).copied();
        let place = textures.key(&slot.to_string());
        match (find(old), find(new)) {
            (Some(old), Some(new)) => {
                changes.field(&place, "texture", &old.texture, &new.texture, text);
            }
            (Some(old), None) => changes.removed(&place, old.texture.to_string()),
            (None, Some(new)) => changes.added(&place, new.texture.to_string()),
            (None, None) => {}
        }
    }
    let floats = Kind {
        key: |float: &crate::FloatParam| float.name,
        name: NameTable::param,
        value: |float| float_text(float.value),
    };
    each(
        changes,
        &at.key("floats"),
        (&old.floats, &new.floats),
        &floats,
        |changes, at, old, new| {
            changes.field(
                at,
                "value",
                &old.value.to_bits(),
                &new.value.to_bits(),
                |bits| float_text(f32::from_bits(*bits)),
            );
        },
    );
    let colors = Kind {
        key: |color: &crate::ColorParam| color.name,
        name: NameTable::param,
        value: |color| colour_text(color.rgba),
    };
    each(
        changes,
        &at.key("colors"),
        (&old.colors, &new.colors),
        &colors,
        |changes, at, old, new| {
            changes.field(at, "rgba", &old.rgba, &new.rgba, |rgba| colour_text(*rgba));
        },
    );
}

fn sorted_slots(old: &SubMaterial, new: &SubMaterial) -> Vec<u8> {
    let mut slots: Vec<u8> = old
        .textures
        .iter()
        .chain(&new.textures)
        .map(|texture| texture.slot)
        .collect();
    slots.sort_unstable();
    slots.dedup();
    slots
}

fn float_text(value: f32) -> String {
    if value.is_finite() {
        value.to_string()
    } else {
        format!("{value} ({:#010x})", value.to_bits())
    }
}

fn colour_text([red, green, blue, alpha]: [u8; 4]) -> String {
    format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}")
}
