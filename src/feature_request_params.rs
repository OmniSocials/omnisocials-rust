//! Typed request parameters for `GET /limits` and `POST /feature-requests`.
//!
//! `GET /limits` lists what OmniSocials cannot do, why, and what to do
//! instead; a feature request goes to the OmniSocials team for the person
//! who owns the key. No scope needed: every key may call these.

use serde::Serialize;

/// Query for `GET /limits`.
#[derive(Debug, Clone, Default)]
pub struct ListLimitsParams {
    /// Only the entries for this platform (entries for every channel stay
    /// in): `instagram`, `facebook`, `linkedin`, `linkedin_page`, `youtube`,
    /// `tiktok`, `x`, `pinterest`, `bluesky`, `threads`, `mastodon`,
    /// `google_business`, `telegram`, `discord`, `reddit`, `snapchat`.
    pub platform: Option<String>,
    /// Only the entries of this area: `posts`, `media`, `inbox`,
    /// `analytics`, `automations`, `accounts`, `ads`, `team`, `other`.
    pub area: Option<String>,
}

/// Body for `POST /feature-requests`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateFeatureRequestParams {
    /// What the person wants to do and why, in one or two sentences (10 to
    /// 1000 characters). Write it yourself; never paste a conversation.
    pub summary: String,
    /// A short name, at most 120 characters. Made from the summary when
    /// left out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The platform it is about, when it is about one (same values as
    /// [`ListLimitsParams::platform`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// The area it is about (same values as [`ListLimitsParams::area`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,
    /// The `id` of the `GET /limits` entry the request is about. A second
    /// person of the same organisation then adds a vote to the open request
    /// instead of a new one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_id: Option<String>,
}
