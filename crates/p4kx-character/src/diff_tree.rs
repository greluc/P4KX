// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use crate::diff::{Changes, Trail, hex, text};
use crate::{ColorParam, FloatParam, ItemPort, Material, NameTable, SubMaterial, Texture};

/// How the items of a list are paired, named and shown when only one file has them.
struct Kind<T> {
    key: fn(&T) -> u32,
    name: for<'n> fn(&'n NameTable, u32) -> Option<&'n str>,
    value: fn(&T) -> String,
    numbered: bool,
}

impl<T> Kind<T> {
    fn label(&self, names: &NameTable, hash: u32) -> String {
        if self.numbered {
            hash.to_string()
        } else {
            (self.name)(names, hash).map_or_else(|| hex(&hash), str::to_owned)
        }
    }
}

struct Paired<'a, T> {
    hash: u32,
    ordinal: usize,
    old: Option<&'a T>,
    new: Option<&'a T>,
    new_at: Option<usize>,
}

fn ordinals<T>(items: &[T], key: fn(&T) -> u32) -> Vec<(u32, usize)> {
    let mut seen: HashMap<u32, usize> = HashMap::with_capacity(items.len());
    items
        .iter()
        .map(|item| {
            let hash = key(item);
            let ordinal = seen.entry(hash).or_default();
            let at = *ordinal;
            *ordinal += 1;
            (hash, at)
        })
        .collect()
}

fn first_free(
    keys: &[(u32, usize)],
    taken: &[bool],
    hash: u32,
    accept: impl Fn(usize) -> bool,
) -> Option<usize> {
    keys.iter()
        .zip(taken)
        .enumerate()
        .find(|(at, ((other, _), taken))| !**taken && *other == hash && accept(*at))
        .map(|(at, _)| at)
}

fn claim(mate: Option<usize>, taken: &mut [bool]) -> Option<usize> {
    if let Some(slot) = mate.and_then(|at| taken.get_mut(at)) {
        *slot = true;
    }
    mate
}

/// Pairs the items of two lists that have the same hash: first those that are equal, then the
/// rest in order. The pairs follow the first list, the items only in the second come last.
fn pair<'a, T: PartialEq>(old: &'a [T], new: &'a [T], key: fn(&T) -> u32) -> Vec<Paired<'a, T>> {
    let old_keys = ordinals(old, key);
    let new_keys = ordinals(new, key);
    let mut taken = vec![false; new.len()];
    let mut mates: Vec<Option<usize>> = Vec::with_capacity(old.len());
    for (item, (hash, _)) in old.iter().zip(&old_keys) {
        let mate = first_free(&new_keys, &taken, *hash, |at| new.get(at) == Some(item));
        mates.push(claim(mate, &mut taken));
    }
    for (mate, (hash, _)) in mates.iter_mut().zip(&old_keys) {
        if mate.is_none() {
            let found = first_free(&new_keys, &taken, *hash, |_| true);
            *mate = claim(found, &mut taken);
        }
    }
    let mut pairs: Vec<_> = old
        .iter()
        .zip(&old_keys)
        .zip(&mates)
        .map(|((item, (hash, ordinal)), mate)| Paired {
            hash: *hash,
            ordinal: *ordinal,
            old: Some(item),
            new: mate.and_then(|at| new.get(at)),
            new_at: *mate,
        })
        .collect();
    pairs.extend(
        new.iter()
            .zip(&new_keys)
            .zip(&taken)
            .filter(|(_, taken)| !**taken)
            .map(|((item, (hash, ordinal)), _)| Paired {
                hash: *hash,
                ordinal: *ordinal,
                old: None,
                new: Some(item),
                new_at: None,
            }),
    );
    pairs
}

fn each<T: PartialEq>(
    changes: &mut Changes<'_>,
    trail: &Trail,
    lists: (&[T], &[T]),
    kind: &Kind<T>,
    both: impl Fn(&mut Changes<'_>, &Trail, &T, &T),
) {
    let pairs = pair(lists.0, lists.1, kind.key);
    for pair in &pairs {
        let label = kind.label(changes.names, pair.hash);
        let at = trail.segment(&label, &hex(&pair.hash), pair.ordinal);
        match (pair.old, pair.new) {
            (Some(old), Some(new)) if old != new => both(changes, &at, old, new),
            (Some(old), None) => changes.removed(&at, (kind.value)(old)),
            (None, Some(new)) => changes.added(&at, (kind.value)(new)),
            _ => {}
        }
    }
    let mut matched: Vec<(usize, &Paired<'_, T>)> = pairs
        .iter()
        .filter_map(|pair| pair.new_at.map(|at| (at, pair)))
        .collect();
    if matched
        .iter()
        .zip(matched.iter().skip(1))
        .any(|(a, b)| a.0 > b.0)
    {
        let names = changes.names;
        let list = |matched: &[(usize, &Paired<'_, T>)]| {
            matched
                .iter()
                .map(|(_, pair)| {
                    let label = kind.label(names, pair.hash);
                    if pair.ordinal == 0 {
                        label
                    } else {
                        format!("{label}#{}", pair.ordinal)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        let old = list(&matched);
        matched.sort_by_key(|(at, _)| *at);
        changes.replaced(&trail.key("order"), old, list(&matched));
    }
}

fn plural(count: u64, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

fn port_text(port: &ItemPort) -> String {
    let children = port.count().saturating_sub(1);
    if children == 0 {
        format!("item {}", port.item)
    } else {
        format!(
            "item {}, {}",
            port.item,
            plural(children, "child port", "child ports")
        )
    }
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
        numbered: false,
    };
    each(
        changes,
        trail,
        (&old.children, &new.children),
        &kind,
        port_fields,
    );
}

fn material_text(material: &Material) -> String {
    format!(
        "base material {}, {}",
        material.base_material,
        plural(
            material.sub_materials.len() as u64,
            "sub-material",
            "sub-materials"
        )
    )
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
        value: material_text,
        numbered: false,
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
        value: |sub| {
            format!(
                "{}, {}, {}",
                plural(sub.textures.len() as u64, "texture", "textures"),
                plural(sub.floats.len() as u64, "float", "floats"),
                plural(sub.colors.len() as u64, "colour", "colours")
            )
        },
        numbered: false,
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
    let textures = Kind {
        key: |texture: &Texture| u32::from(texture.slot),
        name: |_, _| None,
        value: |texture| texture.texture.to_string(),
        numbered: true,
    };
    each(
        changes,
        &at.key("textures"),
        (&old.textures, &new.textures),
        &textures,
        |changes, at, old, new| {
            changes.field(at, "texture", &old.texture, &new.texture, text);
        },
    );
    let floats = Kind {
        key: |float: &FloatParam| float.name,
        name: NameTable::param,
        value: |float| float_text(float.value),
        numbered: false,
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
        key: |color: &ColorParam| color.name,
        name: NameTable::param,
        value: |color| colour_text(color.rgba),
        numbered: false,
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
