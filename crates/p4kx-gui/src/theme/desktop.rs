//! The desktop metrics of the window, taken from `docs/design/p4kx-desktop-spec.json`, which
//! derives them from the KRT design system for a mouse and keyboard tool.

use egui::Color32;

use super::tokens::{FS_SM, FS_XS, REM, SPACE_2, SPACE_3, SPACE_4, SPACE_6, SPACE_8, TOUCH_TARGET};

/// The height of a data row in the table, the tree, the channel list and dropdowns.
pub const ROW_HEIGHT: f32 = 36.0;
/// The horizontal padding of a table cell.
pub const CELL_PADDING_X: f32 = 0.8 * REM;
/// The font size of a data row, the `tree-cell` size.
pub const ROW_FONT: f32 = 0.86 * REM;

/// The height of the header.
pub const HEADER_HEIGHT: f32 = 46.0;
/// The horizontal padding of the header.
pub const HEADER_PADDING_X: f32 = SPACE_4;
/// The size of the brand mark in the header.
pub const BRAND_MARK: f32 = 24.0;
/// The gap between brand mark and wordmark.
pub const BRAND_GAP: f32 = 10.0;
/// The gap between the brand and the menus.
pub const BRAND_PADDING_RIGHT: f32 = 24.0;
/// The font size of the wordmark.
pub const WORDMARK_SIZE: f32 = FS_SM;
/// The horizontal padding of a menu link.
pub const MENU_LINK_PADDING_X: f32 = 14.0;
/// The font size of a menu link.
pub const MENU_LINK_SIZE: f32 = FS_XS;
/// The minimum width of a dropdown.
pub const DROPDOWN_MIN_WIDTH: f32 = 240.0;
/// The horizontal padding of a dropdown entry.
pub const DROPDOWN_ITEM_PADDING_X: f32 = SPACE_3;

/// The default width of the sidebar.
pub const SIDEBAR_WIDTH: f32 = 300.0;
/// The minimum width of the sidebar.
pub const SIDEBAR_MIN_WIDTH: f32 = 240.0;
/// The maximum width of the sidebar.
pub const SIDEBAR_MAX_WIDTH: f32 = 480.0;
/// The padding above a sidebar section title.
pub const SECTION_TITLE_PADDING_TOP: f32 = 14.0;
/// The padding below a sidebar section title.
pub const SECTION_TITLE_PADDING_BOTTOM: f32 = 6.0;
/// The horizontal padding of sidebar rows and titles.
pub const SIDEBAR_PADDING_X: f32 = SPACE_3;
/// The gap between a channel name and its Build.
pub const CHANNEL_GAP: f32 = SPACE_2;
/// The left padding of a tree row at depth zero.
pub const TREE_PADDING_LEFT: f32 = SPACE_2;
/// The indent of a tree row per depth level.
pub const TREE_INDENT: f32 = SPACE_3;
/// The offset of a depth rail inside its level.
pub const TREE_RAIL_OFFSET: f32 = 5.0;
/// The gap between chevron and folder name.
pub const TREE_GAP: f32 = 6.0;
/// The size of a drawn chevron.
pub const CHEVRON: f32 = 12.0;
/// The stroke of a drawn chevron.
pub const CHEVRON_STROKE: f32 = 3.0;

/// The padding and gap of the toolbar.
pub const TOOLBAR_PADDING: f32 = SPACE_3;
/// The flex basis of the search field.
pub const SEARCH_WIDTH: f32 = 260.0;
/// The left padding of the search field, room for its icon.
pub const SEARCH_PADDING_LEFT: f32 = 2.4 * REM;
/// The inset of the search icon.
pub const SEARCH_ICON_INSET: f32 = 0.8 * REM;
/// The size of an inline icon.
pub const ICON: f32 = REM;
/// The height of a quick filter, `krt-segmented--sm`.
pub const SEGMENT_HEIGHT: f32 = 34.0;
/// The horizontal padding of a quick filter.
pub const SEGMENT_PADDING_X: f32 = SPACE_3;

/// The height of a button.
pub const BUTTON_HEIGHT: f32 = TOUCH_TARGET;
/// The horizontal padding of a button.
pub const BUTTON_PADDING_X: f32 = SPACE_4;
/// The height of a compact `btn-xs` button.
pub const BUTTON_XS_HEIGHT: f32 = 32.0;
/// The horizontal padding of a compact button.
pub const BUTTON_XS_PADDING_X: f32 = 0.55 * REM;
/// The font size of a compact button.
pub const BUTTON_XS_SIZE: f32 = 0.68 * REM;
/// The tracking of a button label in em.
pub const BUTTON_TRACKING: f32 = 0.03;

/// The font size of a table head.
pub const TABLE_HEAD_SIZE: f32 = 0.74 * REM;
/// The tracking of a table head in em.
pub const TABLE_HEAD_TRACKING: f32 = 0.07;
/// The width of the checkbox column.
pub const CHECK_COLUMN: f32 = TOUCH_TARGET;
/// The width of the compression column.
pub const COMPRESSION_COLUMN: f32 = 160.0;
/// The width of the encryption column.
pub const ENCRYPTION_COLUMN: f32 = 180.0;
/// The width of the size column.
pub const SIZE_COLUMN: f32 = 130.0;
/// The gap between a head label and its sort chevron.
pub const SORT_GAP: f32 = 6.0;

/// The size of a checkbox or radio.
pub const TOGGLE_SIZE: f32 = 1.2 * REM;
/// The gap between a radio and its label.
pub const TOGGLE_GAP: f32 = SPACE_3;
/// The size of the dot of a selected radio.
pub const RADIO_DOT: f32 = 8.0;

/// The left padding of the tab bar.
pub const TAB_BAR_PADDING_LEFT: f32 = SPACE_3;
/// The horizontal padding of a tab.
pub const TAB_PADDING_X: f32 = 14.0;
/// The font size of a tab.
pub const TAB_SIZE: f32 = 0.72 * REM;
/// The tracking of a tab in em.
pub const TAB_TRACKING: f32 = 0.06;

/// The default height of the preview.
pub const PREVIEW_HEIGHT: f32 = 236.0;
/// The minimum height of the preview.
pub const PREVIEW_MIN_HEIGHT: f32 = 120.0;
/// The height of the path line of the preview.
pub const PREVIEW_PATH_HEIGHT: f32 = 32.0;
/// The horizontal padding inside the preview.
pub const PREVIEW_PADDING_X: f32 = SPACE_4;
/// The height of a hex row.
pub const HEX_ROW_HEIGHT: f32 = 20.0;
/// The width of the offset column of a hex row.
pub const HEX_OFFSET_WIDTH: f32 = 88.0;
/// The width of one byte cell.
pub const HEX_BYTE_WIDTH: f32 = 22.0;
/// The gap after a byte cell.
pub const HEX_BYTE_GAP: f32 = 2.0;
/// The extra gap after eight bytes.
pub const HEX_GROUP_GAP: f32 = 10.0;
/// The gap before the ASCII column.
pub const HEX_ASCII_GAP: f32 = SPACE_4;
/// The width of one ASCII cell.
pub const HEX_ASCII_WIDTH: f32 = 10.0;

/// The height of the status bar.
pub const STATUS_HEIGHT: f32 = 32.0;
/// The horizontal padding of a status bar item.
pub const STATUS_ITEM_PADDING_X: f32 = SPACE_3;
/// The widest the archive path in the status bar gets.
pub const STATUS_PATH_MAX_WIDTH: f32 = 440.0;

/// The padding of the empty state.
pub const EMPTY_PADDING: f32 = SPACE_6;
/// The size of the empty state's icon.
pub const EMPTY_ICON: f32 = 32.0;
/// The font size of the empty state's title and the modal title.
pub const TITLE_SIZE: f32 = 0.85 * REM;
/// The tracking of the empty state's title and the modal title in em.
pub const TITLE_TRACKING: f32 = 0.06;
/// The vertical padding of the notice bar.
pub const NOTICE_PADDING_Y: f32 = 10.0;
/// The size of the notice icon.
pub const NOTICE_ICON: f32 = 20.0;

/// The width of a modal.
pub const MODAL_WIDTH: f32 = 440.0;
/// The width of a wide modal.
pub const MODAL_WIDE_WIDTH: f32 = 600.0;
/// The gap between groups in a modal body.
pub const MODAL_GROUP_GAP: f32 = 20.0;
/// The size of a modal corner bracket.
pub const BRACKET: f32 = 13.0;
/// The stroke of a modal corner bracket.
pub const BRACKET_STROKE: f32 = 2.0;
/// The top edge of a modal.
pub const MODAL_TOP_EDGE: f32 = 3.0;
/// The tracking of a collapsible section title in points.
pub const SECTION_TRACKING: f32 = 1.0;
/// The size of the chevron of a collapsible section.
pub const SECTION_CHEVRON: f32 = 14.0;
/// The tallest a list inside a collapsible section gets.
pub const SECTION_LIST_HEIGHT: f32 = 180.0;
/// The accent bar on the left of a collapsible section or alert.
pub const ACCENT_BAR: f32 = 4.0;
/// The bar that marks the active row or the selected tab.
pub const ACTIVE_BAR: f32 = 3.0;

/// The height of the progress bar.
pub const PROGRESS_HEIGHT: f32 = 8.0;
/// The size of a spinner.
pub const SPINNER: f32 = 20.0;
/// The size of an inline spinner.
pub const SPINNER_INLINE: f32 = 14.0;
/// The ring of a spinner.
pub const SPINNER_RING: f32 = 2.0;
/// One turn of a spinner in seconds.
pub const SPINNER_PERIOD: f64 = 0.9;
/// The narrowest the key column of a detail list gets.
pub const DETAIL_KEY_MIN_WIDTH: f32 = 140.0;
/// The gap between the rows of a detail list.
pub const DETAIL_ROW_GAP: f32 = 0.55 * REM;
/// The gap between the key and the value column of a detail list.
pub const DETAIL_COLUMN_GAP: f32 = 1.25 * REM;
/// The tracking of a detail list key in em.
pub const DETAIL_KEY_TRACKING: f32 = 0.04;
/// The font size of a detail list value.
pub const DETAIL_VALUE_SIZE: f32 = 1.05 * REM;
/// The font size of a `.section-title` inside a panel.
pub const PANEL_TITLE_SIZE: f32 = REM;
/// The space above a `.section-title` that follows other content.
pub const PANEL_TITLE_MARGIN_TOP: f32 = SPACE_8;
/// The space below a `.section-title`.
pub const PANEL_TITLE_MARGIN_BOTTOM: f32 = SPACE_4;
/// The padding between a `.section-title` and its hairline.
pub const PANEL_TITLE_PADDING_BOTTOM: f32 = SPACE_2;
/// The widest a tooltip gets.
pub const TOOLTIP_MAX_WIDTH: f32 = 272.0;
/// The width of a scrollbar.
pub const SCROLLBAR: f32 = 12.0;
/// The width of a thin scrollbar in the sidebar.
pub const SCROLLBAR_THIN: f32 = 8.0;
/// The shortest a scrollbar thumb gets.
pub const SCROLL_THUMB_MIN: f32 = 32.0;

/// The width of the focus ring.
pub const FOCUS_WIDTH: f32 = 2.0;
/// The gap between a control and its focus ring.
pub const FOCUS_OFFSET: f32 = 2.0;
/// The opacity of a disabled control.
pub const DISABLED_OPACITY: f32 = 0.45;
/// The length of a colour transition in seconds.
pub const TRANSITION: f32 = 0.2;

/// The ghost button's hover tint, `rgba(231,126,35,0.07)`.
pub const GHOST_HOVER: Color32 = Color32::from_rgba_unmultiplied_const(231, 126, 35, 18);
/// The outline button's hover tint and the chip tint, `rgba(231,126,35,0.12)`.
pub const OUTLINE_HOVER: Color32 = Color32::from_rgba_unmultiplied_const(231, 126, 35, 31);
/// The zebra stripe of a table, `rgba(255,255,255,0.02)`.
pub const ZEBRA: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 5);
/// The fill of a warning alert, `rgba(255,210,63,0.15)`.
pub const ALERT_WARNING_FILL: Color32 = Color32::from_rgba_unmultiplied_const(255, 210, 63, 38);
/// The fill of the empty state while a file is dragged over it, `rgba(231,126,35,0.10)`.
pub const DROP_TARGET_FILL: Color32 = Color32::from_rgba_unmultiplied_const(231, 126, 35, 26);
/// The track of a spinner, `rgba(231,126,35,0.25)`.
pub const SPINNER_TRACK: Color32 = Color32::from_rgba_unmultiplied_const(231, 126, 35, 64);
/// The flat scrim behind a modal, `rgba(0,0,0,0.8)`.
pub const SCRIM: Color32 = Color32::from_rgba_unmultiplied_const(0, 0, 0, 204);

/// The horizontal padding of an Archive picker.
pub const PICKER_PADDING_X: f32 = SPACE_3;
/// The right padding of an Archive picker, room for its chevron.
pub const PICKER_PADDING_RIGHT: f32 = 2.0 * REM;
/// The inset of the chevron from the right edge of an Archive picker.
pub const PICKER_CHEVRON_INSET: f32 = 0.65 * REM;
/// The narrowest an Archive picker gets.
pub const PICKER_MIN_WIDTH: f32 = SEARCH_WIDTH;

/// The widest the progress bar of a Comparison gets.
pub const COMPARISON_PROGRESS_WIDTH: f32 = 320.0;

/// The horizontal padding of the facts bar.
pub const FACTS_PADDING_X: f32 = SPACE_4;
/// The vertical padding of the facts bar.
pub const FACTS_PADDING_Y: f32 = SPACE_2;
/// The gap between the facts of the facts bar.
pub const FACTS_GAP: f32 = SPACE_4;
/// The font size of a fact's key.
pub const FACT_KEY_SIZE: f32 = 0.62 * REM;
/// The tracking of a fact's key in em.
pub const FACT_KEY_TRACKING: f32 = 0.08;
/// The gap between a fact's key and its value.
pub const FACT_KEY_GAP: f32 = 5.0;
/// The font size of a fact's value.
pub const FACT_VALUE_SIZE: f32 = 0.78 * REM;

/// The horizontal padding of a KPI tile.
pub const KPI_PADDING_X: f32 = 14.0;
/// The vertical padding of a KPI tile.
pub const KPI_PADDING_Y: f32 = 10.0;
/// The font size of a KPI tile's value.
pub const KPI_VALUE_SIZE: f32 = 1.2 * REM;

/// The horizontal padding of a chip.
pub const CHIP_PADDING_X: f32 = SPACE_2;
/// The vertical padding of a chip.
pub const CHIP_PADDING_Y: f32 = 0.15 * REM;
/// The tracking of a chip label in em.
pub const CHIP_TRACKING: f32 = 0.06;
/// The fill of a success chip, `rgba(35,158,51,0.12)`.
pub const CHIP_SUCCESS_FILL: Color32 = Color32::from_rgba_unmultiplied_const(35, 158, 51, 31);
/// The fill of a danger chip, `rgba(163,0,10,0.15)`.
pub const CHIP_DANGER_FILL: Color32 = Color32::from_rgba_unmultiplied_const(163, 0, 10, 38);
/// The fill of a warning chip, `rgba(255,210,63,0.12)`.
pub const CHIP_WARNING_FILL: Color32 = Color32::from_rgba_unmultiplied_const(255, 210, 63, 31);

/// The width of the change column.
pub const CHANGE_COLUMN: f32 = 130.0;
/// The width of a size column of the change list.
pub const CHANGE_SIZE_COLUMN: f32 = 150.0;
/// The width of a count column of the directory table.
pub const COUNT_COLUMN: f32 = 110.0;

/// The height of a line and a hunk head of the line diff, the hex row height.
pub const DIFF_ROW_HEIGHT: f32 = HEX_ROW_HEIGHT;
/// The font size of the line diff.
pub const DIFF_FONT: f32 = FS_XS;
/// The horizontal padding around the line numbers of the line diff, the cell padding.
pub const DIFF_GUTTER_PADDING_X: f32 = CELL_PADDING_X;
/// The gap between the marker and the text of a line of the line diff.
pub const DIFF_MARKER_GAP: f32 = SPACE_2;
/// The fill of an added line, the success chip fill.
pub const DIFF_ADDED_FILL: Color32 = CHIP_SUCCESS_FILL;
/// The fill of a removed line, the danger chip fill.
pub const DIFF_REMOVED_FILL: Color32 = CHIP_DANGER_FILL;

/// The fill of a danger alert and of the marked error in the Query editor, `rgba(163,0,10,0.2)`.
pub const ALERT_DANGER_FILL: Color32 = Color32::from_rgba_unmultiplied_const(163, 0, 10, 51);
/// The underline of the marked error in the Query editor.
pub const ERROR_UNDERLINE: f32 = 2.0;
/// The rows of text the Query editor shows.
pub const QUERY_EDITOR_ROWS: usize = 4;
/// The horizontal padding of a textarea.
pub const TEXTAREA_PADDING_X: f32 = 0.75 * REM;
/// The vertical padding of a textarea.
pub const TEXTAREA_PADDING_Y: f32 = SPACE_2;
/// The line height of a textarea, as a factor of its font size.
pub const TEXTAREA_LINE_HEIGHT: f32 = 1.5;

/// The gap between the parts of the pagination.
pub const PAGE_GAP: f32 = 0.4 * REM;
/// The horizontal padding of a page button.
pub const PAGE_BUTTON_PADDING_X: f32 = SPACE_2;
/// The horizontal padding of the page status.
pub const PAGE_STATUS_PADDING_X: f32 = 0.75 * REM;
/// The opacity of a disabled page button.
pub const PAGE_DISABLED_OPACITY: f32 = 0.4;

/// The initial width of a column of the Query result.
pub const QUERY_COLUMN_WIDTH: f32 = 200.0;
/// The narrowest a column of the Query result gets.
pub const QUERY_COLUMN_MIN_WIDTH: f32 = 80.0;
