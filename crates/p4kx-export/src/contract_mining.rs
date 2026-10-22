use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::{LocalisedText, every_field_required};
use crate::guid_text::CatalogGuid;

/// A mineable element: a `MineableElement` Record under `mining/mineableelements`. Several
/// elements may link to the same `ResourceType`, so none is picked for a commodity.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct MineableElement<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `agricium_ore`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The GUID of the `ResourceType` Record the Reference `resourceType` links to. It equals a
    /// commodity's `subtypeGuid` only when the catalog has that commodity (20 of the 41 linked
    /// `ResourceType`s on LIVE). `null` for a null or dangling Reference.
    pub(crate) resource_type_guid: Option<CatalogGuid>,
    /// `elementInstability`, from 0 to 1000 on LIVE; `null` when the Record states none.
    pub(crate) instability: Option<f32>,
    /// `elementResistance`, from -1 to 1 on LIVE.
    pub(crate) resistance: Option<f32>,
    /// `elementOptimalWindowMidpoint`, from 0.3 to 1 on LIVE.
    pub(crate) optimal_window_midpoint: Option<f32>,
    /// `elementOptimalWindowMidpointRandomness`, from 0 to 0.3 on LIVE.
    pub(crate) optimal_window_midpoint_randomness: Option<f32>,
    /// `elementOptimalWindowThinness`, from -1 to 10 on LIVE.
    pub(crate) optimal_window_thinness: Option<f32>,
    /// `elementExplosionMultiplier`, from -1000 to 260 on LIVE.
    pub(crate) explosion_multiplier: Option<f32>,
    /// `elementClusterFactor`, from 0 to 1 on LIVE.
    pub(crate) cluster_factor: Option<f32>,
}

/// A refining process: a `RefiningProcess` Record under `refiningprocess`, one method of a
/// refinery. The DataForge holds no yields.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct RefiningProcess<'a> {
    /// The GUID of the Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name without its type, e.g. `processingtype-fastcareful`.
    pub(crate) class_name: &'a str,
    /// The record path, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The name of the method, from `processName`. The localisation key and the texts are game
    /// texts.
    #[schemars(extend("x-gameText" = true))]
    pub(crate) process_name: LocalisedText<'a>,
    /// `refiningSpeed`, one of `Slow`, `Normal` and `Fast` on LIVE; `null` when the Record states
    /// none. The schema keeps a string, as a new value is additive.
    pub(crate) refining_speed: Option<&'a str>,
    /// `refiningQuality`, one of `Wasteful`, `Normal` and `Careful` on LIVE; `null` when the Record
    /// states none. The schema keeps a string, as a new value is additive.
    pub(crate) refining_quality: Option<&'a str>,
}
