// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::StructValue;

use crate::reader::{float, text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Align {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Flex {
    pub(crate) column: bool,
    pub(crate) reverse: bool,
    pub(crate) wrap: bool,
    pub(crate) justify: Justify,
    pub(crate) align: Align,
    pub(crate) gap: f32,
}

pub(crate) fn parse_flex(layout: &StructValue<'_>) -> (Option<Flex>, Option<String>) {
    if layout.type_name() != "BuildingBlocks_FlexContainer" {
        return (None, Some(layout.type_name().to_owned()));
    }
    let direction = text(layout, "direction").unwrap_or("Row");
    let column = direction.contains("Column");
    let wrap = text(layout, "wrap").is_some_and(|wrap| wrap.starts_with("Wrap"));
    let justify = match text(layout, "axisJustification")
        .unwrap_or("Start")
        .to_ascii_lowercase()
        .as_str()
    {
        "center" => Justify::Center,
        "end" | "flexend" => Justify::End,
        "spacebetween" => Justify::SpaceBetween,
        "spacearound" => Justify::SpaceAround,
        "spaceevenly" => Justify::SpaceEvenly,
        _ => Justify::Start,
    };
    let align = match text(layout, "itemAlignment")
        .unwrap_or("Start")
        .to_ascii_lowercase()
        .as_str()
    {
        "center" => Align::Center,
        "end" | "flexend" => Align::End,
        "stretch" => Align::Stretch,
        _ => Align::Start,
    };
    let spacing = if column {
        "rowSpacing"
    } else {
        "columnSpacing"
    };
    (
        Some(Flex {
            column,
            reverse: direction.contains("Reverse"),
            wrap,
            justify,
            align,
            gap: float(layout, spacing).unwrap_or(0.0),
        }),
        None,
    )
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Item {
    pub(crate) size: [f32; 2],
    pub(crate) grow: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Slot {
    pub(crate) position: [f32; 2],
    pub(crate) size: [f32; 2],
}

fn at(pair: [f32; 2], axis: usize) -> f32 {
    pair.get(axis).copied().unwrap_or(0.0)
}

fn set(pair: &mut [f32; 2], axis: usize, value: f32) {
    if let Some(slot) = pair.get_mut(axis) {
        *slot = value;
    }
}

fn number(count: usize) -> f32 {
    f32::from(u16::try_from(count).unwrap_or(u16::MAX))
}

pub(crate) fn arrange(flex: &Flex, content: [f32; 2], items: &[Item]) -> Vec<Slot> {
    let main = usize::from(flex.column);
    let cross = 1 - main;
    let count = number(items.len());
    let gaps = flex.gap * (count - 1.0).max(0.0);
    let used = items.iter().map(|item| at(item.size, main)).sum::<f32>() + gaps;
    let free = at(content, main) - used;
    let grow_total: f32 = items.iter().map(|item| item.grow.max(0.0)).sum();
    let (grown, spare) = if free > 0.0 && grow_total > 0.0 {
        (free / grow_total, 0.0)
    } else {
        (0.0, free.max(0.0))
    };
    let (lead, between) = match flex.justify {
        Justify::Center => (spare / 2.0, 0.0),
        Justify::End => (spare, 0.0),
        Justify::SpaceBetween if count > 1.0 => (0.0, spare / (count - 1.0)),
        Justify::Start | Justify::SpaceBetween => (0.0, 0.0),
        Justify::SpaceAround => (spare / count.max(1.0) / 2.0, spare / count.max(1.0)),
        Justify::SpaceEvenly => (spare / (count + 1.0), spare / (count + 1.0)),
    };
    let mut cursor = lead;
    let place = |item: &Item, cursor: &mut f32| {
        let mut size = item.size;
        let stretched = grown.mul_add(item.grow.max(0.0), at(size, main));
        set(&mut size, main, stretched);
        let room = at(content, cross) - at(size, cross);
        let across = match flex.align {
            Align::Start => 0.0,
            Align::Center => room / 2.0,
            Align::End => room,
            Align::Stretch => {
                set(&mut size, cross, at(content, cross));
                0.0
            }
        };
        let mut position = [0.0; 2];
        set(&mut position, main, *cursor);
        set(&mut position, cross, across);
        *cursor += at(size, main) + flex.gap + between;
        Slot { position, size }
    };
    if flex.reverse {
        let mut slots: Vec<Slot> = items
            .iter()
            .rev()
            .map(|item| place(item, &mut cursor))
            .collect();
        slots.reverse();
        slots
    } else {
        items.iter().map(|item| place(item, &mut cursor)).collect()
    }
}
