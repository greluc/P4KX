// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Response, Ui};

use super::button::{Kind, Size, button_state, laid_label};

/// Shows a ghost button of `size` with `label` that says whether it is `selected`, with the
/// selected colours of the controls and the selected state for assistive technology.
pub(crate) fn choice(ui: &mut Ui, size: Size, label: &str, selected: bool) -> Response {
    let galley = laid_label(ui, size, label);
    button_state(
        ui,
        (Kind::Ghost, size),
        (label, galley),
        (true, Some(selected)),
    )
}
