//! The metrics of the Query editor's completion list and of the saved Queries, taken from the
//! `queryCompletion` and `savedQueries` entries of `docs/design/p4kx-desktop-spec.json`; they are
//! re-exported by [`super::desktop`].

use super::desktop::{DROPDOWN_MIN_WIDTH, SECTION_LIST_HEIGHT};
use super::tokens::REM;

/// The tallest the completion list of the Query editor gets before it scrolls, the
/// `.combobox__listbox` `max-height` of 18 rem.
pub const COMPLETION_MAX_HEIGHT: f32 = 18.0 * REM;
/// The padding of a `.combobox__notice`.
pub const COMBOBOX_NOTICE_PADDING: f32 = 0.75 * REM;
/// The tallest the list of saved Queries gets before it scrolls, the list height of a
/// collapsible section.
pub const SAVED_LIST_HEIGHT: f32 = SECTION_LIST_HEIGHT;
/// The widest the name of a saved Query gets before it ends in an ellipsis, a dropdown's width.
pub const SAVED_NAME_MAX_WIDTH: f32 = DROPDOWN_MIN_WIDTH;
