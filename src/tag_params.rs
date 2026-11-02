//! Typed request parameters for post tags (`/tags`, `GET /analytics/tags`).
//!
//! Tags are the workspace's own labels for its posts (a hook style, a
//! format) to filter and compare posts in Analytics. They never go to a
//! platform.

use serde::Serialize;

/// One tag in a list of tags: a tag id, or a tag name. A name matches
/// case-insensitively; a new name in `tags` on a post, or in `add` / `set`
/// of [`ApplyTagsParams`], becomes a tag of the workspace.
///
/// Build it with `.into()`: `12.into()` or `"Question".into()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum TagRef {
    /// A tag id.
    Id(i64),
    /// A tag name.
    Name(String),
}

impl From<i64> for TagRef {
    fn from(value: i64) -> Self {
        TagRef::Id(value)
    }
}

impl From<i32> for TagRef {
    fn from(value: i32) -> Self {
        TagRef::Id(i64::from(value))
    }
}

impl From<u32> for TagRef {
    fn from(value: u32) -> Self {
        TagRef::Id(i64::from(value))
    }
}

impl From<&str> for TagRef {
    fn from(value: &str) -> Self {
        TagRef::Name(value.to_owned())
    }
}

impl From<String> for TagRef {
    fn from(value: String) -> Self {
        TagRef::Name(value)
    }
}

/// Body for `POST /tags`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateTagParams {
    /// Unique in the workspace (case-insensitive), 1 to 40 characters.
    pub name: String,
    /// One of `blue`, `green`, `orange`, `red`, `purple`, `pink`, `teal`,
    /// `yellow`, `brown`, `gray`. Omit to get the next colour of the palette.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Body for `PATCH /tags/:id`. A field left `None` keeps its value.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateTagParams {
    /// New name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New colour (see [`CreateTagParams::color`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Body for `POST /tags/apply`: change the tags of up to 500 posts at once,
/// published ones included.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ApplyTagsParams {
    /// The posts to change, at most 500. Ids that are not posts of this
    /// workspace come back in `data.not_found`.
    pub post_ids: Vec<String>,
    /// Tags to add; the others stay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<TagRef>>,
    /// Tags to take off.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove: Option<Vec<TagRef>>,
    /// Replaces each post's tags. `Some(vec![])` removes every tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set: Option<Vec<TagRef>>,
}

/// Query for `GET /analytics/tags`.
#[derive(Debug, Clone, Default)]
pub struct TagAnalyticsParams {
    /// `"YYYY-MM-DD"`, `"YYYY-MM"` or ISO 8601. Default: 90 days ago.
    pub start_date: Option<String>,
    /// `"YYYY-MM-DD"`, `"YYYY-MM"` or ISO 8601 (inclusive). Default: today.
    pub end_date: Option<String>,
    /// Only this channel's numbers (`"instagram"`, `"tiktok"`, ...).
    /// Default: every connected channel.
    pub platform: Option<String>,
    /// Measured posts a tag needs before `enough_posts` is true (1 to 50,
    /// default 5).
    pub min_posts: Option<u32>,
}
