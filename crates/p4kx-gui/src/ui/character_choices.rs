// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Label, Rect, Role, ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2};
use p4kx_assembly::BodyVariant;
use p4kx_settings::ModelMaterials;

use super::search::eyebrow;
use super::{Dialog, Requests};
use crate::theme::character_metrics::CHARACTER_EXPRESSION_ROWS;
use crate::theme::desktop::{
    AUDIO_CONTROL_GAP, BUTTON_XS_HEIGHT, MODAL_GROUP_GAP, ROW_FONT, ROW_HEIGHT, SPINNER_INLINE,
    TREE_PADDING_LEFT,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, SPACE_2, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames;
use crate::widgets::indicator::spinner;
use crate::widgets::slider::{Steps, slider};
use crate::widgets::tab;
use crate::widgets::text::{line, paint_left, plain};
use crate::widgets::toggle::{checkbox, radio};
use crate::{
    CHARACTER_LODS, CharacterText as T, CharactersTab, ChooserKind, EntityText, Model, Texts,
};

const BODIES: [BodyVariant; 2] = [BodyVariant::Male, BodyVariant::Female];
const MATERIALS: [ModelMaterials; 2] = [ModelMaterials::Colours, ModelMaterials::Textures];
const STRENGTH: Steps = Steps {
    step: 0.05,
    page: 0.25,
};

enum Ask {
    Body(BodyVariant),
    HeadFromArchive,
    HeadFromFile,
    HeadRemove,
    LoadoutChoose,
    LoadoutClear,
    PieceAdd,
    PieceRemove(usize),
    Expressions(bool),
    Expression(Option<usize>),
    Strength(f32),
    Lod(u32),
    Materials(ModelMaterials),
    Export,
}

/// Shows the choices of the Character: the body, the head, the Loadout and the pieces, the
/// expressions and the export.
pub(super) fn show(ui: &mut Ui, model: &mut Model, requests: &mut Requests) {
    let ask = {
        let (tab, texts) = model.characters_with_texts();
        let tab: &CharactersTab = tab;
        frames::preview_body()
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = ROW_HEIGHT / 6.0;
                [
                    body(ui, tab, texts),
                    head(ui, tab, texts),
                    armour(ui, tab, texts),
                    expressions(ui, tab, texts),
                    export(ui, tab, texts),
                ]
                .into_iter()
                .flatten()
                .next()
            })
            .inner
    };
    if let Some(ask) = ask {
        apply(ui, model, ask, requests);
    }
}

fn apply(ui: &Ui, model: &mut Model, ask: Ask, requests: &mut Requests) {
    match ask {
        Ask::Body(variant) => model.characters_mut().set_body(variant),
        Ask::HeadFromArchive => {
            model.begin_character_choice(ChooserKind::Head);
        }
        Ask::HeadFromFile => requests.dialog = Some(Dialog::CharacterChf),
        Ask::HeadRemove => model.characters_mut().clear_head(),
        Ask::LoadoutChoose => {
            model.begin_character_choice(ChooserKind::Loadout);
        }
        Ask::LoadoutClear => model.characters_mut().clear_loadout(),
        Ask::PieceAdd => {
            model.begin_character_choice(ChooserKind::Piece);
        }
        Ask::PieceRemove(position) => model.characters_mut().remove_piece(position),
        Ask::Expressions(enabled) => model.characters_mut().set_expressions_enabled(enabled),
        Ask::Expression(position) => model.characters_mut().choose_expression(position),
        Ask::Strength(weight) => model.characters_mut().set_expression_weight(weight),
        Ask::Lod(level) => model.characters_mut().set_lod(level),
        Ask::Materials(materials) => model.characters_mut().set_materials(materials),
        Ask::Export => requests.dialog = Some(Dialog::CharacterFile),
    }
    ui.ctx().request_repaint();
}

fn heading(ui: &mut Ui, texts: &Texts, text: impl Into<crate::Text>) {
    ui.add_space(MODAL_GROUP_GAP / 2.0);
    ui.label(eyebrow(&texts.get(text)));
}

fn muted(ui: &mut Ui, text: String) {
    ui.add(Label::new(plain(text, FS_XS, Weight::Regular).color(palette::GRAY_2_TEXT)).wrap());
}

fn body(ui: &mut Ui, tab: &CharactersTab, texts: &Texts) -> Option<Ask> {
    heading(ui, texts, T::BodyHeading);
    let labels = [texts.get(T::BodyMale), texts.get(T::BodyFemale)];
    let active = BODIES.iter().position(|variant| *variant == tab.body());
    let locked = tab.head().is_some();
    let clicked = ui
        .add_enabled_ui(!locked, |ui| ui.horizontal(|ui| tab::segments(ui, &labels, active)).inner)
        .inner;
    if locked {
        muted(ui, texts.get(T::BodyFromHead));
    }
    clicked
        .and_then(|at| BODIES.get(at))
        .map(|variant| Ask::Body(*variant))
}

fn head(ui: &mut Ui, tab: &CharactersTab, texts: &Texts) -> Option<Ask> {
    heading(ui, texts, T::HeadHeading);
    let mut ask = None;
    if tab.is_reading_head() {
        ui.horizontal(|ui| {
            spinner(ui, SPINNER_INLINE);
            muted(ui, texts.get(T::HeadReading));
        });
    } else if let Some(head) = tab.head() {
        let args = [
            ("name", head.name().to_owned().into()),
            ("rig", head.rig().to_owned().into()),
        ];
        ui.add(Label::new(plain(texts.format(T::HeadLine, &args), FS_XS, Weight::Regular)).wrap());
    } else {
        muted(ui, texts.get(T::HeadNone));
    }
    if let Some(reason) = tab.head_failure() {
        let text = texts.format(T::HeadFailed, &[("reason", reason.to_owned().into())]);
        ui.add(
            Label::new(plain(text, FS_XS, Weight::Regular).color(palette::DANGER_TEXT)).wrap(),
        );
    }
    let listed = tab.sources().is_some_and(|sources| !sources.heads().is_empty());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = AUDIO_CONTROL_GAP;
        let archive = texts.get(T::HeadFromArchive);
        if button::button(ui, Kind::Outline, Size::Compact, &archive, listed).clicked() {
            ask = Some(Ask::HeadFromArchive);
        }
        let file = texts.get(T::HeadFromFile);
        if button::button(ui, Kind::Outline, Size::Compact, &file, true).clicked() {
            ask = Some(Ask::HeadFromFile);
        }
        if tab.head().is_some() {
            let remove = texts.get(T::HeadRemove);
            if button::button(ui, Kind::Ghost, Size::Compact, &remove, true).clicked() {
                ask = Some(Ask::HeadRemove);
            }
        }
    });
    ask
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn armour(ui: &mut Ui, tab: &CharactersTab, texts: &Texts) -> Option<Ask> {
    heading(ui, texts, T::ArmourHeading);
    let mut ask = None;
    match tab.loadout() {
        Some(path) => {
            let args = [("name", file_name(path).to_owned().into())];
            ui.add(
                Label::new(plain(texts.format(T::LoadoutLine, &args), FS_XS, Weight::Regular))
                    .wrap(),
            );
        }
        None => muted(ui, texts.get(T::LoadoutNone)),
    }
    let listed = tab
        .sources()
        .is_some_and(|sources| !sources.loadouts().is_empty());
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = AUDIO_CONTROL_GAP;
        let choose = texts.get(T::LoadoutChoose);
        if button::button(ui, Kind::Outline, Size::Compact, &choose, listed).clicked() {
            ask = Some(Ask::LoadoutChoose);
        }
        if tab.loadout().is_some() {
            let clear = texts.get(T::LoadoutClear);
            if button::button(ui, Kind::Ghost, Size::Compact, &clear, true).clicked() {
                ask = Some(Ask::LoadoutClear);
            }
        }
    });
    if tab.pieces().is_empty() {
        muted(ui, texts.get(T::PiecesNone));
    }
    ui.spacing_mut().item_spacing.y = 0.0;
    for (position, piece) in tab.pieces().iter().enumerate() {
        if piece_row(ui, texts, piece.name()) {
            ask = Some(Ask::PieceRemove(position));
        }
    }
    ui.spacing_mut().item_spacing.y = ROW_HEIGHT / 6.0;
    let add = texts.get(T::PieceAdd);
    if button::button(ui, Kind::Outline, Size::Compact, &add, true).clicked() {
        ask = Some(Ask::PieceAdd);
    }
    ask
}

fn piece_row(ui: &mut Ui, texts: &Texts, name: &str) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::ListItem, true, name));
    let side = BUTTON_XS_HEIGHT;
    let icon = Rect::from_center_size(
        pos2(rect.right() - side / 2.0, rect.center().y),
        vec2(side, side),
    );
    let label = texts.format(T::PieceRemove, &[("name", name.to_owned().into())]);
    let removed = button::close_icon(ui, icon, &label).clicked();
    let inner = Rect::from_min_max(
        pos2(rect.left() + TREE_PADDING_LEFT + SPACE_2, rect.top()),
        pos2(icon.left() - SPACE_2, rect.bottom()),
    );
    let galley = line(
        ui,
        plain(name, ROW_FONT, Weight::Regular),
        inner.width().max(0.0),
    );
    paint_left(ui, inner, galley, palette::GRAY_1);
    removed
}

fn expressions(ui: &mut Ui, tab: &CharactersTab, texts: &Texts) -> Option<Ask> {
    heading(ui, texts, T::ExpressionsHeading);
    if tab.head().is_none() {
        muted(ui, texts.get(T::ExpressionsNoHead));
        return None;
    }
    let mut ask = None;
    let mut enabled = tab.expressions_enabled();
    if checkbox(ui, &mut enabled, &texts.get(T::ExpressionsLoad)).changed() {
        ask = Some(Ask::Expressions(enabled));
    }
    if !enabled {
        return ask;
    }
    let names = tab.expressions();
    if names.is_empty() {
        if !tab.is_building() {
            muted(ui, texts.get(T::ExpressionsNone));
        }
        return ask;
    }
    let chosen = tab.expression();
    ScrollArea::vertical()
        .id_salt("characters-expressions")
        .max_height(ROW_HEIGHT * CHARACTER_EXPRESSION_ROWS)
        .auto_shrink([false, true])
        .show_rows(ui, ROW_HEIGHT, names.len() + 1, |ui, range| {
            for row in range {
                let position = row.checked_sub(1);
                let name = position.map_or_else(
                    || texts.get(T::ExpressionNeutral),
                    |at| names.get(at).map_or_else(String::new, |one| one.name().to_owned()),
                );
                if radio(ui, chosen == position, &name, true).clicked() {
                    ask = Some(Ask::Expression(position));
                }
            }
        });
    let percent = format!("{:.0}", tab.expression_weight() * 100.0);
    let label = texts.format(T::ExpressionStrength, &[("percent", percent.clone().into())]);
    ui.label(plain(label.as_str(), FS_XS, Weight::Regular).color(palette::GRAY_1));
    let width = ui.available_width();
    let (_, picked) = slider(
        ui,
        (&label, &percent),
        tab.expression_weight(),
        STRENGTH,
        width,
    );
    picked.map(Ask::Strength).or(ask)
}

fn export(ui: &mut Ui, tab: &CharactersTab, texts: &Texts) -> Option<Ask> {
    heading(ui, texts, T::ExportHeading);
    let mut ask = None;
    ui.label(plain(texts.get(EntityText::ViewLod), FS_XS, Weight::Regular).color(palette::GRAY_1));
    let levels = CHARACTER_LODS.map(|level| level.to_string());
    let active = CHARACTER_LODS.iter().position(|level| *level == tab.lod());
    if let Some(level) = tab::segments(ui, &levels, active).and_then(|at| CHARACTER_LODS.get(at)) {
        ask = Some(Ask::Lod(*level));
    }
    ui.label(
        plain(texts.get(EntityText::ViewMaterials), FS_XS, Weight::Regular)
            .color(palette::GRAY_1),
    );
    for materials in MATERIALS {
        let label = texts.get(match materials {
            ModelMaterials::Colours => EntityText::ViewColours,
            ModelMaterials::Textures => EntityText::ViewTextures,
        });
        if radio(ui, tab.materials() == materials, &label, true).clicked() {
            ask = Some(Ask::Materials(materials));
        }
    }
    let action = texts.get(T::ExportAction);
    if button::button(ui, Kind::Cta, Size::Normal, &action, tab.export_available()).clicked() {
        ask = Some(Ask::Export);
    }
    ask
}
