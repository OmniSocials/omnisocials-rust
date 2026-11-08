//! Typed request parameters for X statistics on credits: the `x_statistics`
//! change of `PATCH /accounts/{id}`, and the X statistics webhook event
//! names.
//!
//! X bills every statistics read, and OmniSocials passes X's price on as
//! credits: half a credit per post check (plus half a credit for a video or
//! GIF) and 1 credit per follower count.

use serde::Serialize;

/// The X statistics webhook events: `x_statistics.paused` (the
/// organisation's X statistics stopped at 0 credits) and
/// `x_statistics.resumed` (they started again after a top-up). One delivery
/// per workspace with X; `data` is `{ workspace_id, account_id, reason }`
/// with `reason` `insufficient_credits` or `credits_added`.
pub const X_STATISTICS_WEBHOOK_EVENTS: [&str; 2] = ["x_statistics.paused", "x_statistics.resumed"];

/// Body for `PATCH /accounts/{id}`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateAccountParams {
    /// The new X statistics setting. Always sent: `None` goes out as
    /// `"x_statistics": null` (OmniSocials Connect keys only: the account
    /// follows its customer's setting again).
    pub x_statistics: Option<XStatisticsInput>,
}

/// The X statistics change. Set one field or more; a `None` field is left
/// out and keeps its value.
#[derive(Debug, Clone, Default, Serialize)]
pub struct XStatisticsInput {
    /// How often each post is checked: `full` (5 checks: about 5 hours, day
    /// 1, 3 and 7, and day 30 for posts with 100+ views), `light` (2
    /// checks: about 5 hours and day 7), `off`, or `custom` (OmniSocials
    /// Connect keys only, with `post_schedule`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub posts: Option<String>,
    /// How often the follower count is read: `daily`, `weekly` or `off`;
    /// OmniSocials Connect keys also `hourly` and `6h`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followers: Option<String>,
    /// OmniSocials Connect keys only, with `posts: "custom"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_schedule: Option<XStatisticsPostSchedule>,
}

/// A custom check schedule (OmniSocials Connect keys, with `posts:
/// "custom"`).
#[derive(Debug, Clone, Default, Serialize)]
pub struct XStatisticsPostSchedule {
    /// Times after publishing, such as `"1h"`, `"1d"`, `"7d"` (at most 24,
    /// 1h to 365d, at least 1 hour apart).
    pub at: Vec<String>,
    /// Repeat after the last one (1d to 30d).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub then_every: Option<String>,
    /// Stop repeating after this age.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
}
