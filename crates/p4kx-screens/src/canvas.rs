// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::rc::Rc;

use p4kx_dataforge::{DataForge, Guid, RecordTree, StructValue};

use crate::bindings::{TextBinding, text_bindings};
use crate::error::ScreenError;
use crate::reader::{as_struct, child, float, items, reference, text};
use crate::style::Entry;
use crate::style::parse_entries;
use crate::widget::{Parent, Widget};

pub(crate) const CANVAS_TYPE: &str = "BuildingBlocks_Canvas";

#[derive(Debug, Clone)]
pub(crate) struct StyleRef {
    pub(crate) guid: Guid,
    pub(crate) record_path: String,
}

#[derive(Debug)]
pub(crate) struct BrandStyle {
    pub(crate) style: StyleRef,
    pub(crate) entries: Rc<[Entry]>,
}

#[derive(Debug)]
pub(crate) struct Canvas {
    pub(crate) guid: Guid,
    pub(crate) record_path: String,
    pub(crate) size: [f32; 2],
    pub(crate) unit: f32,
    pub(crate) widgets: Vec<Widget>,
    pub(crate) roots: Vec<usize>,
    pub(crate) children: Vec<Vec<usize>>,
    pub(crate) brands: Vec<BrandStyle>,
    pub(crate) shared: Vec<StyleRef>,
    pub(crate) defaults: Rc<[Entry]>,
    pub(crate) embedded: Rc<[Entry]>,
    pub(crate) unreachable: Vec<usize>,
    pub(crate) text_bindings: HashMap<usize, TextBinding>,
}

impl Canvas {
    pub(crate) fn pick_brand(&self, code: Option<&str>) -> Option<&BrandStyle> {
        let wanted = code.map(|code| format!("s_{}_", code.to_ascii_lowercase()));
        wanted
            .as_deref()
            .and_then(|wanted| {
                self.brands.iter().find(|brand| {
                    brand
                        .style
                        .record_path
                        .to_ascii_lowercase()
                        .contains(wanted)
                })
            })
            .or_else(|| self.brands.first())
    }

    pub(crate) fn read(tree: &RecordTree<'_>) -> Self {
        let root = tree.root();
        let guid = tree.guid();
        let size = child(root, "size").map_or([0.0; 2], |size| {
            [
                float(size, "x").unwrap_or(0.0),
                float(size, "y").unwrap_or(0.0),
            ]
        });
        let raw = text(root, "coordinateMethod") == Some("useRaw");
        let unit = if raw { size[1] } else { 1.0 };
        let widgets: Vec<Widget> = items(root, "scene")
            .iter()
            .filter_map(as_struct)
            .map(|widget| {
                let mut widget = Widget::parse(widget, guid);
                if raw {
                    widget.scale_units(unit);
                }
                widget
            })
            .collect();
        let (roots, children) = hierarchy(&widgets);
        let unreachable = unreachable_widgets(&widgets, &roots, &children);
        Self {
            unreachable,
            text_bindings: text_bindings(root),
            guid,
            record_path: tree.record_path().to_owned(),
            size,
            unit,
            roots,
            children,
            widgets,
            brands: brand_styles(root),
            shared: shared_styles(root),
            defaults: child(root, "defaultStyles")
                .map(|defaults| parse_entries(items(defaults, "entries")).into())
                .unwrap_or_default(),
            embedded: parse_entries(items(root, "embeddedStyles")).into(),
        }
    }
}

fn unreachable_widgets(widgets: &[Widget], roots: &[usize], children: &[Vec<usize>]) -> Vec<usize> {
    let mut seen = vec![false; widgets.len()];
    let mut stack: Vec<usize> = roots.to_vec();
    while let Some(index) = stack.pop() {
        if let Some(flag) = seen.get_mut(index) {
            if *flag {
                continue;
            }
            *flag = true;
        }
        stack.extend(children.get(index).into_iter().flatten().copied());
    }
    widgets
        .iter()
        .enumerate()
        .filter(|(index, widget)| {
            widget.parent != Parent::Library && !seen.get(*index).copied().unwrap_or(true)
        })
        .map(|(index, _)| index)
        .collect()
}

fn hierarchy(widgets: &[Widget]) -> (Vec<usize>, Vec<Vec<usize>>) {
    let mut roots = Vec::new();
    let mut children = vec![Vec::new(); widgets.len()];
    for (index, widget) in widgets.iter().enumerate() {
        match widget.parent {
            Parent::Library => {}
            Parent::Widget(parent) if parent != index && parent < widgets.len() => {
                if let Some(list) = children.get_mut(parent) {
                    list.push(index);
                }
            }
            Parent::Root | Parent::Widget(_) => roots.push(index),
        }
    }
    let order = |index: &usize| widgets.get(*index).map_or(0, |widget| widget.order);
    roots.sort_by_key(order);
    for list in &mut children {
        list.sort_by_key(order);
    }
    (roots, children)
}

fn style_ref(structure: &StructValue<'_>, name: &str) -> Option<StyleRef> {
    reference(structure, name).map(|style| StyleRef {
        guid: style.guid(),
        record_path: style.record_path().to_owned(),
    })
}

fn brand_styles(root: &StructValue<'_>) -> Vec<BrandStyle> {
    items(root, "brandStyles")
        .iter()
        .filter_map(as_struct)
        .filter_map(|brand| {
            Some(BrandStyle {
                style: style_ref(brand, "brandIdentifier")?,
                entries: parse_entries(items(brand, "entries")).into(),
            })
        })
        .collect()
}

fn shared_styles(root: &StructValue<'_>) -> Vec<StyleRef> {
    let defaults =
        child(root, "defaultStyles").and_then(|defaults| style_ref(defaults, "sharedStyles"));
    defaults
        .into_iter()
        .chain(style_ref(root, "style"))
        .collect()
}

pub(crate) struct CanvasStore<'a, 'f> {
    forge: &'a DataForge<'f>,
    cache: HashMap<Guid, Option<Rc<Canvas>>>,
}

impl<'a, 'f> CanvasStore<'a, 'f> {
    pub(crate) fn new(forge: &'a DataForge<'f>) -> Self {
        Self {
            forge,
            cache: HashMap::new(),
        }
    }

    pub(crate) const fn forge(&self) -> &'a DataForge<'f> {
        self.forge
    }

    pub(crate) fn get(&mut self, guid: Guid) -> Option<Rc<Canvas>> {
        if let Some(cached) = self.cache.get(&guid) {
            return cached.clone();
        }
        let canvas = self
            .forge
            .record_by_guid(guid)
            .and_then(|record| record.tree().ok())
            .filter(|tree| tree.type_name() == CANVAS_TYPE)
            .map(|tree| Rc::new(Canvas::read(&tree)));
        self.cache.insert(guid, canvas.clone());
        canvas
    }
}

pub(crate) fn find_canvas(forge: &DataForge<'_>, text: &str) -> Result<Guid, ScreenError> {
    let unknown = || ScreenError::UnknownCanvas {
        text: text.to_owned(),
    };
    if let Ok(guid) = text.trim().parse::<Guid>() {
        let record = forge.record_by_guid(guid).ok_or_else(unknown)?;
        return if record.type_name().is_ok_and(|name| name == CANVAS_TYPE) {
            Ok(guid)
        } else {
            Err(ScreenError::NotACanvas {
                path: record.record_path().unwrap_or_default().to_owned(),
                type_name: record.type_name().unwrap_or_default().to_owned(),
            })
        };
    }
    let path = text.trim().replace('\\', "/");
    let path = path.strip_suffix(".xml").unwrap_or(&path);
    let matches: Vec<Guid> = forge
        .records_at_path(path)
        .filter(|record| record.type_name().is_ok_and(|name| name == CANVAS_TYPE))
        .map(|record| record.guid())
        .collect();
    match matches.as_slice() {
        [] => Err(unknown()),
        [guid] => Ok(*guid),
        many => Err(ScreenError::AmbiguousCanvas {
            text: text.to_owned(),
            count: many.len(),
        }),
    }
}
