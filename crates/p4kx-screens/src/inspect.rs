// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{BTreeMap, HashSet};

use p4kx_dataforge::Guid;
use serde::Serialize;

use crate::assets::{AssetSet, ScreenAssets};
use crate::canvas::CanvasStore;
use crate::elements::is_rendered;

const MAX_CANVASES: usize = 4096;

/// How many widgets of one element type a screen holds and whether P4KX draws them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ElementCount {
    /// The element type, e.g. `WidgetText`, without the `BuildingBlocks_` prefix.
    pub element: String,
    /// The widgets of this type, in every canvas of the screen.
    pub count: usize,
    /// Whether the renderer draws this type.
    pub rendered: bool,
}

pub(crate) fn inspect(
    store: &mut CanvasStore<'_, '_>,
    root: Guid,
) -> (ScreenAssets, Vec<ElementCount>) {
    let mut assets = AssetSet::default();
    let mut kinds: BTreeMap<String, (usize, bool)> = BTreeMap::new();
    let mut seen = HashSet::new();
    let mut queue = vec![root];
    while let Some(guid) = queue.pop() {
        if !seen.insert(guid) || seen.len() > MAX_CANVASES {
            continue;
        }
        let Some(canvas) = store.get(guid) else {
            continue;
        };
        assets.add_canvas(&canvas.record_path);
        for widget in &canvas.widgets {
            assets.add(&widget.assets);
            let name = widget
                .type_name
                .strip_prefix("BuildingBlocks_")
                .unwrap_or(&widget.type_name);
            kinds
                .entry(name.to_owned())
                .or_insert_with(|| (0, is_rendered(&widget.type_name)))
                .0 += 1;
            queue.extend(widget.canvas);
        }
    }
    let elements = kinds
        .into_iter()
        .map(|(element, (count, rendered))| ElementCount {
            element,
            count,
            rendered,
        })
        .collect();
    (assets.finish(), elements)
}
