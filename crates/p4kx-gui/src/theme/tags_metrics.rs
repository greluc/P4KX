//! The metrics of the tag tree of the Tags tab, taken from the `tagTree` entry of
//! `docs/design/p4kx-desktop-spec.json`; they sit beside [`super::desktop`], which is at its size limit.

/// The width of the GUID column of the tag tree.
pub const TAGS_GUID_WIDTH: f32 = 300.0;
/// The width of the legacy ID column of the tag tree.
pub const TAGS_LEGACY_WIDTH: f32 = 150.0;
/// The narrowest the tag column of the tag tree gets before the table scrolls sideways: room for
/// the name below twelve levels of rails.
pub const TAGS_NAME_MIN_WIDTH: f32 = 320.0;
