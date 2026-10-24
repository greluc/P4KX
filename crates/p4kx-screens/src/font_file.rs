// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::compose::ScreenComposer;
use crate::reader::text;

const UI_ELEMENT: &str = "libs/foundry/records/ui/uielements/buildingblocks";
const FALLBACK: &str = "UI/fonts/Shared/fonts_en.gfx";

impl ScreenComposer<'_, '_> {
    /// Returns the path of the movie that holds the fonts of the screens, as the `fontFile` of
    /// the `UIElement` Record `uielements/buildingblocks` names it, `UI/fonts/Shared/fonts_en.gfx`
    /// when the Archive has no such Record; the Archive holds it below `Data/`.
    #[must_use]
    pub fn font_file(&self) -> String {
        self.store
            .forge()
            .records_at_path(UI_ELEMENT)
            .next()
            .and_then(|record| record.tree().ok())
            .and_then(|tree| text(tree.root(), "fontFile").map(str::to_owned))
            .filter(|path| !path.is_empty())
            .unwrap_or_else(|| FALLBACK.to_owned())
    }
}
