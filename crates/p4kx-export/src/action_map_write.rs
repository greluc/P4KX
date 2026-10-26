// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;
use std::io::{self, Write};

use p4kx_extraction::{CSV_BYTE_ORDER_MARK, Language, TextSet, decode_line_breaks, write_csv_cell};
use serde::Serialize;

use crate::action_map::{Action, ActionMap, ActionMaps, Binding, Device};

/// The file format of an action map export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActionMapFormat {
    /// One JSON object: the `language` and the `action_maps` with their actions and bindings.
    Json,
    /// CSV with a UTF-8 byte order mark and CRLF line ends, one row per action; a text cell that
    /// a spreadsheet would run as a formula starts with `'`.
    Csv,
    /// As [`Csv`](Self::Csv) with the exact texts and no `'`; the byte order mark stays.
    CsvRaw,
}

/// What an action map export wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActionMapCounts {
    /// The action maps written.
    pub action_maps: usize,
    /// The actions written.
    pub actions: usize,
    /// The labels and map labels whose key has no text in the language.
    pub unresolved_labels: usize,
}

/// Writes the action maps with the labels in `language`.
///
/// A label is the text of its localisation key with `\n` decoded to a line feed; a key without a
/// text in `language` leaves the label empty in CSV and `null` in JSON. The CSV has the columns
/// `action_map`, `action_map_label`, `action`, `label`, `activation_mode`, `keyboard`, `mouse`,
/// `gamepad` and `joystick`; a device cell joins its bindings with ` | `, and a binding whose
/// activation mode differs from the action's carries it in parentheses. The JSON states every
/// binding with its device, input and activation mode.
///
/// # Errors
///
/// Returns the [`io::Error`] of the first failed write.
pub fn write_action_maps(
    maps: &ActionMaps,
    texts: &TextSet<'_>,
    language: Language,
    format: ActionMapFormat,
    out: &mut impl Write,
) -> io::Result<ActionMapCounts> {
    let labels = Labels {
        texts,
        language,
        unresolved: std::cell::Cell::new(0),
    };
    match format {
        ActionMapFormat::Json => write_json(maps, &labels, out)?,
        ActionMapFormat::Csv => write_csv(maps, &labels, out, true)?,
        ActionMapFormat::CsvRaw => write_csv(maps, &labels, out, false)?,
    }
    Ok(ActionMapCounts {
        action_maps: maps.maps.len(),
        actions: maps.action_count(),
        unresolved_labels: labels.unresolved.get(),
    })
}

struct Labels<'a, 't> {
    texts: &'a TextSet<'t>,
    language: Language,
    unresolved: std::cell::Cell<usize>,
}

impl Labels<'_, '_> {
    fn resolve(&self, key: Option<&str>) -> Option<Cow<'_, str>> {
        let key = key?;
        let found = self
            .texts
            .lookup(key, self.language, None)
            .text()
            .map(decode_line_breaks);
        if found.is_none() {
            self.unresolved.set(self.unresolved.get() + 1);
        }
        found
    }
}

#[derive(Serialize)]
struct JsonFile<'a> {
    language: &'static str,
    action_maps: Vec<JsonMap<'a>>,
}

#[derive(Serialize)]
struct JsonMap<'a> {
    name: &'a str,
    label_key: Option<&'a str>,
    label: Option<Cow<'a, str>>,
    actions: Vec<JsonAction<'a>>,
}

#[derive(Serialize)]
struct JsonAction<'a> {
    name: &'a str,
    label_key: Option<&'a str>,
    label: Option<Cow<'a, str>>,
    activation_mode: Option<&'a str>,
    bindings: Vec<JsonBinding<'a>>,
}

#[derive(Serialize)]
struct JsonBinding<'a> {
    device: &'static str,
    input: &'a str,
    activation_mode: Option<&'a str>,
}

fn write_json(maps: &ActionMaps, labels: &Labels<'_, '_>, out: &mut impl Write) -> io::Result<()> {
    let file = JsonFile {
        language: labels.language.code(),
        action_maps: maps
            .maps
            .iter()
            .map(|map| JsonMap {
                name: &map.name,
                label_key: map.label_key.as_deref(),
                label: labels.resolve(map.label_key.as_deref()),
                actions: map
                    .actions
                    .iter()
                    .map(|action| JsonAction {
                        name: &action.name,
                        label_key: action.label_key.as_deref(),
                        label: labels.resolve(action.label_key.as_deref()),
                        activation_mode: action.activation_mode.as_deref(),
                        bindings: action
                            .bindings
                            .iter()
                            .map(|binding| JsonBinding {
                                device: binding.device.name(),
                                input: &binding.input,
                                activation_mode: binding.activation_mode.as_deref(),
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
    };
    serde_json::to_writer(&mut *out, &file)?;
    out.write_all(b"\n")
}

fn write_csv(
    maps: &ActionMaps,
    labels: &Labels<'_, '_>,
    out: &mut impl Write,
    neutralise: bool,
) -> io::Result<()> {
    out.write_all(CSV_BYTE_ORDER_MARK)?;
    out.write_all(b"action_map,action_map_label,action,label,activation_mode")?;
    for device in Device::ALL {
        out.write_all(b",")?;
        out.write_all(device.name().as_bytes())?;
    }
    out.write_all(b"\r\n")?;
    let mut cell = String::new();
    for map in &maps.maps {
        let map_label = labels.resolve(map.label_key.as_deref());
        for action in &map.actions {
            let row = Row {
                map,
                map_label: map_label.as_deref(),
                action,
                label: labels.resolve(action.label_key.as_deref()),
            };
            write_row(out, &row, &mut cell, neutralise)?;
        }
    }
    Ok(())
}

struct Row<'a> {
    map: &'a ActionMap,
    map_label: Option<&'a str>,
    action: &'a Action,
    label: Option<Cow<'a, str>>,
}

fn write_row(
    out: &mut impl Write,
    row: &Row<'_>,
    cell: &mut String,
    neutralise: bool,
) -> io::Result<()> {
    let action = row.action;
    let leading = [
        row.map.name.as_str(),
        row.map_label.unwrap_or_default(),
        action.name.as_str(),
        row.label.as_deref().unwrap_or_default(),
        action.activation_mode.as_deref().unwrap_or_default(),
    ];
    for (position, text) in leading.into_iter().enumerate() {
        if position > 0 {
            out.write_all(b",")?;
        }
        write_csv_cell(out, text, neutralise)?;
    }
    for device in Device::ALL {
        out.write_all(b",")?;
        cell.clear();
        for binding in action.bindings.iter().filter(|b| b.device == device) {
            if !cell.is_empty() {
                cell.push_str(" | ");
            }
            push_binding(cell, binding, action);
        }
        write_csv_cell(out, cell, neutralise)?;
    }
    out.write_all(b"\r\n")
}

fn push_binding(cell: &mut String, binding: &Binding, action: &Action) {
    cell.push_str(&binding.input);
    if let Some(mode) = &binding.activation_mode
        && binding.activation_mode != action.activation_mode
    {
        cell.push_str(" (");
        cell.push_str(mode);
        cell.push(')');
    }
}
