// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::flex::{Flex, Item, arrange};
use crate::paint::Area;
use crate::widget::{Dim, Widget};

pub(crate) struct Sized {
    pub(crate) size: [f32; 2],
    pub(crate) auto: bool,
}

pub(crate) fn size_of(widget: &Widget, parent: [f32; 2], has_children: bool) -> Sized {
    let [parent_width, parent_height] = parent;
    let [width, height] = widget.size;
    let independent = |dim: Dim, axis: f32| match dim {
        Dim::Fixed(value) => Some(value),
        Dim::Percent(value) => Some(value * axis),
        Dim::Auto | Dim::OfWidth(_) | Dim::OfHeight(_) => None,
    };
    let auto_axis = |axis: f32| if has_children { axis } else { 0.0 };
    let fixed_width = independent(width, parent_width);
    let fixed_height = independent(height, parent_height);
    let resolved_width = fixed_width.unwrap_or_else(|| match width {
        Dim::OfHeight(value) => value * fixed_height.unwrap_or(parent_height),
        Dim::OfWidth(value) => value * parent_width,
        _ => auto_axis(parent_width),
    });
    let resolved_height = fixed_height.unwrap_or_else(|| match height {
        Dim::OfWidth(value) => value * resolved_width,
        Dim::OfHeight(value) => value * parent_height,
        _ => auto_axis(parent_height),
    });
    Sized {
        size: [resolved_width.max(0.0), resolved_height.max(0.0)],
        auto: has_children && (width == Dim::Auto || height == Dim::Auto),
    }
}

pub(crate) struct Sibling<'w> {
    pub(crate) widget: &'w Widget,
    pub(crate) index: usize,
    pub(crate) has_children: bool,
}

pub(crate) struct Placed {
    pub(crate) index: usize,
    pub(crate) area: Area,
    pub(crate) auto: bool,
}

fn flow_items(siblings: &[Sibling<'_>], sized: &[Sized], in_flow: &[usize]) -> Vec<Item> {
    in_flow
        .iter()
        .filter_map(|&at| Some((siblings.get(at)?, sized.get(at)?)))
        .map(|(sibling, sized)| {
            let margin = sibling.widget.margin;
            Item {
                size: [
                    sized.size[0].max(0.0) + margin.left + margin.right,
                    sized.size[1].max(0.0) + margin.top + margin.bottom,
                ],
                grow: sibling.widget.grow,
            }
        })
        .collect()
}

pub(crate) fn place(content: Area, flex: Option<&Flex>, siblings: &[Sibling<'_>]) -> Vec<Placed> {
    let parent = [content.width, content.height];
    let sized: Vec<Sized> = siblings
        .iter()
        .map(|sibling| size_of(sibling.widget, parent, sibling.has_children))
        .collect();
    let in_flow: Vec<usize> = (0..siblings.len())
        .filter(|&at| {
            flex.is_some()
                && siblings
                    .get(at)
                    .is_some_and(|sibling| sibling.widget.in_flow)
        })
        .collect();
    let items = flow_items(siblings, &sized, &in_flow);
    let slots = flex
        .map(|flex| arrange(flex, parent, &items))
        .unwrap_or_default();
    siblings
        .iter()
        .zip(&sized)
        .enumerate()
        .map(|(at, (sibling, sized))| {
            let widget = sibling.widget;
            let margin = widget.margin;
            let slot = in_flow
                .iter()
                .position(|&flowed| flowed == at)
                .and_then(|position| slots.get(position));
            let (origin, size) = slot.map_or(([0.0, 0.0], sized.size), |slot| {
                (
                    slot.position,
                    [
                        slot.size[0] - margin.left - margin.right,
                        slot.size[1] - margin.top - margin.bottom,
                    ],
                )
            });
            Placed {
                index: sibling.index,
                area: Area {
                    x: content.x + origin[0] + margin.left + widget.offset[0],
                    y: content.y + origin[1] + margin.top + widget.offset[1],
                    width: size[0].max(0.0),
                    height: size[1].max(0.0),
                },
                auto: sized.auto,
            }
        })
        .collect()
}
