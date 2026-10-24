// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::ops::Range;

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
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Align {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wrap {
    No,
    Yes,
    Reverse,
}

/// A container that lays its children out in lines along one axis: a flex container, a stack or
/// a grid pack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Flex {
    pub(crate) column: bool,
    pub(crate) reverse: bool,
    pub(crate) wrap: Wrap,
    pub(crate) infinite: bool,
    pub(crate) justify: Justify,
    pub(crate) lines: Justify,
    pub(crate) align: Align,
    pub(crate) column_gap: f32,
    pub(crate) row_gap: f32,
}

impl Flex {
    const fn main_gap(&self) -> f32 {
        if self.column {
            self.row_gap
        } else {
            self.column_gap
        }
    }

    const fn cross_gap(&self) -> f32 {
        if self.column {
            self.column_gap
        } else {
            self.row_gap
        }
    }

    /// Returns how the lines are placed across: the start of a container that wraps in reverse
    /// is the end of the cross axis.
    const fn line_justification(&self) -> Justify {
        match (self.wrap, self.lines) {
            (Wrap::Reverse, Justify::Start) => Justify::End,
            (Wrap::Reverse, Justify::End) => Justify::Start,
            (_, how) => how,
        }
    }

    /// Replaces the column and row spacing a style sets.
    pub(crate) const fn apply_gaps(&mut self, [column, row]: [Option<f32>; 2]) {
        if let Some(gap) = column {
            self.column_gap = gap;
        }
        if let Some(gap) = row {
            self.row_gap = gap;
        }
    }
}

fn justify(name: Option<&str>) -> Justify {
    match name.unwrap_or("Start").to_ascii_lowercase().as_str() {
        "center" => Justify::Center,
        "end" | "flexend" => Justify::End,
        "spacebetween" => Justify::SpaceBetween,
        "spacearound" => Justify::SpaceAround,
        "spaceevenly" => Justify::SpaceEvenly,
        "stretch" => Justify::Stretch,
        _ => Justify::Start,
    }
}

fn align(name: Option<&str>) -> Align {
    match name.unwrap_or("Start").to_ascii_lowercase().as_str() {
        "center" => Align::Center,
        "end" | "flexend" => Align::End,
        "stretch" => Align::Stretch,
        _ => Align::Start,
    }
}

const FLEX: &str = "BuildingBlocks_FlexContainer";
const STACK: &str = "BuildingBlocks_StackContainer";
const GRID: &str = "BuildingBlocks_GridPackContainer";

/// Reads a layout policy: the container it makes, or the name of a policy that has no
/// counterpart here.
pub(crate) fn parse_flex(layout: &StructValue<'_>) -> (Option<Flex>, Option<String>) {
    let spacing = |name| float(layout, name).unwrap_or(0.0);
    let vertical = text(layout, "direction") == Some("Vertical");
    let flex = match layout.type_name() {
        FLEX => {
            let direction = text(layout, "direction").unwrap_or("Row");
            let wrap = text(layout, "wrap").unwrap_or("NoWrap");
            Flex {
                column: direction.contains("Column"),
                reverse: direction.contains("Reverse"),
                wrap: match wrap {
                    "Wrap" => Wrap::Yes,
                    "WrapReverse" => Wrap::Reverse,
                    _ => Wrap::No,
                },
                infinite: wrap == "NoWrapInfinite",
                justify: justify(text(layout, "axisJustification")),
                lines: justify(text(layout, "crossAxisJustification")),
                align: align(text(layout, "itemAlignment")),
                column_gap: spacing("columnSpacing"),
                row_gap: spacing("rowSpacing"),
            }
        }
        STACK => Flex {
            column: vertical,
            reverse: false,
            wrap: Wrap::No,
            infinite: true,
            justify: Justify::Start,
            lines: Justify::Start,
            align: Align::Start,
            column_gap: spacing("spacing"),
            row_gap: spacing("spacing"),
        },
        GRID => Flex {
            column: vertical,
            reverse: false,
            wrap: Wrap::Yes,
            infinite: false,
            justify: Justify::Start,
            lines: Justify::Start,
            align: Align::Start,
            column_gap: spacing("gutter"),
            row_gap: spacing("gutter"),
        },
        other => return (None, Some(other.to_owned())),
    };
    (Some(flex), None)
}

/// A child of a container with its outer size, margins included, and how it may grow, shrink
/// and be bounded along the axes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Item {
    pub(crate) size: [f32; 2],
    pub(crate) grow: f32,
    pub(crate) shrink: f32,
    pub(crate) min: [f32; 2],
    pub(crate) max: [f32; 2],
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

/// Holds `value` between `min` and `max`, the minimum winning, and above zero.
pub(crate) const fn limit(value: f32, min: f32, max: f32) -> f32 {
    value.min(max).max(min).max(0.0)
}

fn break_lines(flex: &Flex, items: &[Item], main: usize, extent: f32) -> Vec<Range<usize>> {
    if flex.wrap == Wrap::No || items.is_empty() {
        return std::iter::once(0..items.len()).collect();
    }
    let mut lines = Vec::new();
    let mut start = 0;
    let mut filled = 0.0;
    for (index, item) in items.iter().enumerate() {
        let size = at(item.size, main);
        let gap = if index > start { flex.main_gap() } else { 0.0 };
        if index > start && filled + gap + size > extent {
            lines.push(start..index);
            start = index;
            filled = size;
        } else {
            filled += gap + size;
        }
    }
    lines.push(start..items.len());
    lines
}

fn distribute(
    flex: &Flex,
    items: &[Item],
    line: &Range<usize>,
    main: usize,
    extent: f32,
) -> Vec<f32> {
    let members = items.get(line.clone()).unwrap_or(&[]);
    let mut sizes: Vec<f32> = members.iter().map(|item| at(item.size, main)).collect();
    let gaps = flex.main_gap() * (number(members.len()) - 1.0).max(0.0);
    let free = extent - sizes.iter().sum::<f32>() - gaps;
    if free > 0.0 {
        let total: f32 = members.iter().map(|item| item.grow.max(0.0)).sum();
        let stretch = flex.justify == Justify::Stretch && total <= 0.0;
        for (size, item) in sizes.iter_mut().zip(members) {
            let share = if stretch {
                free / number(members.len())
            } else if total > 0.0 {
                free * item.grow.max(0.0) / total
            } else {
                0.0
            };
            *size = limit(*size + share, at(item.min, main), at(item.max, main));
        }
    } else if free < 0.0 && !flex.infinite {
        let weights: Vec<f32> = members
            .iter()
            .zip(&sizes)
            .map(|(item, size)| item.shrink.max(0.0) * size)
            .collect();
        let total: f32 = weights.iter().sum();
        if total > 0.0 {
            for ((size, item), weight) in sizes.iter_mut().zip(members).zip(weights) {
                *size = limit(
                    *size + free * weight / total,
                    at(item.min, main),
                    at(item.max, main),
                );
            }
        }
    }
    sizes
}

fn spread(how: Justify, spare: f32, count: usize) -> (f32, f32) {
    let spare = spare.max(0.0);
    let count = number(count);
    match how {
        Justify::Center => (spare / 2.0, 0.0),
        Justify::End => (spare, 0.0),
        Justify::SpaceBetween if count > 1.0 => (0.0, spare / (count - 1.0)),
        Justify::SpaceAround => (spare / count.max(1.0) / 2.0, spare / count.max(1.0)),
        Justify::SpaceEvenly => (spare / (count + 1.0), spare / (count + 1.0)),
        Justify::Start | Justify::SpaceBetween | Justify::Stretch => (0.0, 0.0),
    }
}

/// Places `items` in the box `content` of a container: breaks them into lines, grows and
/// shrinks them along the main axis, spreads them by the axis justification, and puts the lines
/// across by the cross justification and each item inside its line by the item alignment.
pub(crate) fn arrange(flex: &Flex, content: [f32; 2], items: &[Item]) -> Vec<Slot> {
    let main = usize::from(flex.column);
    let cross = 1 - main;
    let lines = break_lines(flex, items, main, at(content, main));
    let mut placed = vec![
        Slot {
            position: [0.0; 2],
            size: [0.0; 2],
        };
        items.len()
    ];
    let mut spans: Vec<f32> = Vec::with_capacity(lines.len());
    let mut mains: Vec<Vec<f32>> = Vec::with_capacity(lines.len());
    for line in &lines {
        let sizes = distribute(flex, items, line, main, at(content, main));
        let members = items.get(line.clone()).unwrap_or(&[]);
        spans.push(
            members
                .iter()
                .map(|item| at(item.size, cross))
                .fold(0.0_f32, f32::max),
        );
        mains.push(sizes);
    }
    let line_gap = flex.cross_gap() * (number(lines.len()) - 1.0).max(0.0);
    let free = at(content, cross) - spans.iter().sum::<f32>() - line_gap;
    if flex.lines == Justify::Stretch && free > 0.0 {
        let share = free / number(lines.len());
        for span in &mut spans {
            *span += share;
        }
    }
    let free = at(content, cross) - spans.iter().sum::<f32>() - line_gap;
    let (lead, between) = spread(flex.line_justification(), free, lines.len());
    let order: Vec<usize> = if flex.wrap == Wrap::Reverse {
        (0..lines.len()).rev().collect()
    } else {
        (0..lines.len()).collect()
    };
    let mut cursor = lead;
    for line_at in order {
        let (Some(line), Some(sizes), Some(&span)) =
            (lines.get(line_at), mains.get(line_at), spans.get(line_at))
        else {
            continue;
        };
        place_line(
            flex,
            items,
            line,
            sizes,
            (cursor, span),
            at(content, main),
            &mut placed,
        );
        cursor += span + flex.cross_gap() + between;
    }
    placed
}

fn place_line(
    flex: &Flex,
    items: &[Item],
    line: &Range<usize>,
    sizes: &[f32],
    (line_start, span): (f32, f32),
    extent: f32,
    placed: &mut [Slot],
) {
    let main = usize::from(flex.column);
    let cross = 1 - main;
    let used = flex.main_gap().mul_add(
        (number(sizes.len()) - 1.0).max(0.0),
        sizes.iter().sum::<f32>(),
    );
    let (lead, between) = spread(flex.justify, extent - used, sizes.len());
    let indices: Vec<usize> = if flex.reverse {
        line.clone().rev().collect()
    } else {
        line.clone().collect()
    };
    let mut cursor = lead;
    for index in indices {
        let (Some(item), Some(&along)) = (items.get(index), sizes.get(index - line.start)) else {
            continue;
        };
        let mut size = item.size;
        set(&mut size, main, along);
        let room = span - at(size, cross);
        let across = match flex.align {
            Align::Start => 0.0,
            Align::Center => room / 2.0,
            Align::End => room,
            Align::Stretch => {
                set(
                    &mut size,
                    cross,
                    limit(span, at(item.min, cross), at(item.max, cross)),
                );
                0.0
            }
        };
        let mut position = [0.0; 2];
        set(&mut position, main, cursor);
        set(&mut position, cross, line_start + across);
        cursor += along + flex.main_gap() + between;
        if let Some(slot) = placed.get_mut(index) {
            *slot = Slot { position, size };
        }
    }
}
