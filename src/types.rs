//! Typed request parameters for the OmniSocials API.
//!
//! Field names match the API's snake_case JSON exactly (ported from the
//! canonical MCP client, `mcp-server/src/client.ts`). Every `Option` field is
//! skipped when `None`, so `..Default::default()` produces minimal bodies.
//!
//! Deep platform-specific option blocks (`instagram`, `youtube`, `x`, ...)
//! are [`serde_json::Value`] passthrough on purpose; build them with
//! [`serde_json::json!`].

use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;

// ─── Shared shapes ───────────────────────────────────────────────────────────

/// Post caption: either a single string used everywhere, or a per-platform
/// map with a `default` key (e.g. `{"default": "...", "x": "..."}`).
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Content {
    /// One caption for every platform.
    Text(String),
    /// Per-platform captions; include a `default` key as the fallback.
    PerPlatform(HashMap<String, String>),
}

impl Default for Content {
    fn default() -> Self {
        Content::Text(String::new())
    }
}

impl From<&str> for Content {
    fn from(value: &str) -> Self {
        Content::Text(value.to_owned())
    }
}

impl From<String> for Content {
    fn from(value: String) -> Self {
        Content::Text(value)
    }
}

impl From<HashMap<String, String>> for Content {
    fn from(value: HashMap<String, String>) -> Self {
        Content::PerPlatform(value)
    }
}

/// One media entry, optionally carrying alt text (accessibility description,
/// max 1500 chars). Alt text is delivered to Mastodon (media description),
/// Bluesky (embed alt), X (photo/GIF media metadata), and Pinterest (pin
/// alt_text fallback). Plain and alt-carrying entries can be mixed in the
/// same list.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum MediaEntry {
    /// A plain public URL (in `media_urls`) or Media Library id (in `media_ids`).
    Plain(String),
    /// A public URL plus alt text; serializes as `{"url": ..., "alt": ...}`.
    /// Use in `media_urls`.
    Url {
        /// The public media URL.
        url: String,
        /// Alt text / accessibility description (max 1500 chars).
        #[serde(skip_serializing_if = "Option::is_none")]
        alt: Option<String>,
    },
    /// A Media Library id plus alt text; serializes as `{"id": ..., "alt": ...}`.
    /// Use in `media_ids`.
    Id {
        /// The Media Library id.
        id: String,
        /// Alt text / accessibility description (max 1500 chars).
        #[serde(skip_serializing_if = "Option::is_none")]
        alt: Option<String>,
    },
}

impl From<&str> for MediaEntry {
    fn from(value: &str) -> Self {
        MediaEntry::Plain(value.to_owned())
    }
}

impl From<String> for MediaEntry {
    fn from(value: String) -> Self {
        MediaEntry::Plain(value)
    }
}

/// Media attachments: either a list shared by all platforms, or a
/// per-platform map (e.g. `{"instagram": ["id1"], "x": ["id2"]}`). The
/// `*Entries` variants accept [`MediaEntry`] values for per-media alt text.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum MediaRefs {
    /// The same media for every platform.
    Shared(Vec<String>),
    /// Per-platform media lists.
    PerPlatform(HashMap<String, Vec<String>>),
    /// The same media for every platform, with optional per-media alt text.
    SharedEntries(Vec<MediaEntry>),
    /// Per-platform media lists with optional per-media alt text.
    PerPlatformEntries(HashMap<String, Vec<MediaEntry>>),
}

impl From<Vec<String>> for MediaRefs {
    fn from(value: Vec<String>) -> Self {
        MediaRefs::Shared(value)
    }
}

impl From<Vec<&str>> for MediaRefs {
    fn from(value: Vec<&str>) -> Self {
        MediaRefs::Shared(value.into_iter().map(str::to_owned).collect())
    }
}

impl From<HashMap<String, Vec<String>>> for MediaRefs {
    fn from(value: HashMap<String, Vec<String>>) -> Self {
        MediaRefs::PerPlatform(value)
    }
}

impl From<Vec<MediaEntry>> for MediaRefs {
    fn from(value: Vec<MediaEntry>) -> Self {
        MediaRefs::SharedEntries(value)
    }
}

impl From<HashMap<String, Vec<MediaEntry>>> for MediaRefs {
    fn from(value: HashMap<String, Vec<MediaEntry>>) -> Self {
        MediaRefs::PerPlatformEntries(value)
    }
}

/// An Instagram user tag positioned on an image.
#[derive(Debug, Clone, Serialize)]
pub struct UserTag {
    /// Instagram username to tag (without the `@`).
    pub username: String,
    /// Horizontal position, 0.0 (left) to 1.0 (right).
    pub x: f64,
    /// Vertical position, 0.0 (top) to 1.0 (bottom).
    pub y: f64,
    /// Which carousel image to tag (0-based). Omit for single images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_index: Option<u32>,
}

// ─── Posts ───────────────────────────────────────────────────────────────────

/// Body for `POST /posts/create` and `POST /posts/create-and-publish`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreatePostParams {
    /// Caption. Either a single string or a per-platform map with a `default` key.
    pub content: Content,
    /// Platform identifiers, e.g. `["instagram", "linkedin", "x"]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<String>>,
    /// ISO 8601 datetime. Omit to create a draft. Ignored by
    /// `create_and_publish` (which publishes immediately).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<String>,
    /// Media Library ids, either shared or a per-platform map. Use
    /// [`MediaEntry::Id`] entries for per-media alt text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_ids: Option<MediaRefs>,
    /// Public media URLs, either shared or a per-platform map. Use
    /// [`MediaEntry::Url`] entries for per-media alt text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_urls: Option<MediaRefs>,
    /// Post type, e.g. `"post"`, `"story"`, `"reel"`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Free-form source label for where the post came from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Link to attach (platforms with link previews).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_url: Option<String>,
    /// Title for the attached link preview.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_title: Option<String>,
    /// Description for the attached link preview.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_description: Option<String>,
    /// Thumbnail URL for the attached link preview.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_thumbnail_url: Option<String>,
    /// Instagram location tag (a Facebook Place id; see `locations().search`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_id: Option<String>,
    /// Instagram collaborator usernames.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collaborators: Option<Vec<String>>,
    /// Instagram user tags positioned on the image(s).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_tags: Option<Vec<UserTag>>,
    /// Name of a saved hashtag set (case-insensitive). Applies the set once
    /// at create time; tags already in a caption are skipped; Instagram's
    /// 30-hashtag cap returns error code `hashtag_limit_exceeded`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashtag_set: Option<String>,
    /// Id of a saved hashtag set to apply at create time (see `hashtag_set`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashtag_set_id: Option<String>,
    /// Where the hashtags go: `"caption_append"` (default) or
    /// `"first_comment"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashtag_placement: Option<String>,
    /// Restrict the hashtags to a subset of `channels`. Omit for all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashtag_platforms: Option<Vec<String>>,
    /// Pinterest options (e.g. `json!({"board_id": "...", "title": "..."})`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinterest: Option<Value>,
    /// YouTube options (e.g. `json!({"title": "...", "privacy": "public"})`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub youtube: Option<Value>,
    /// Instagram options (e.g. `json!({"share_to_feed": true})`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instagram: Option<Value>,
    /// Facebook options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facebook: Option<Value>,
    /// LinkedIn personal-profile options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin: Option<Value>,
    /// LinkedIn company-page options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_page: Option<Value>,
    /// TikTok options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiktok: Option<Value>,
    /// X (Twitter) options. Provide 2-25 `thread_parts` (each with `text`,
    /// max 280 chars, and optional `media_ids`/`media_urls`, max 4) to
    /// publish a thread; also supports `reply_settings`, `paid_partnership`,
    /// `made_with_ai`. Thread-part media entries accept `{"url"|"id": ...,
    /// "alt": ...}` objects for per-media alt text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<Value>,
    /// Bluesky options. Same `thread_parts` shape as `x` (300 chars per part).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bluesky: Option<Value>,
    /// Mastodon options. Same `thread_parts` shape as `x` (500 chars per part).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mastodon: Option<Value>,
    /// Google Business options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_business: Option<Value>,
}

/// Body for `PATCH /posts/:id`.
///
/// For `x`, `bluesky`, and `mastodon`: passing `"thread_parts": null` inside
/// the value explicitly clears thread mode (reverts to a single post), while
/// omitting the field leaves the existing thread untouched.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdatePostParams {
    /// New caption. Either a single string or a per-platform map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    /// New ISO 8601 schedule time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<String>,
    /// New platform identifiers list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<String>>,
    /// New Media Library ids, either shared or a per-platform map. Use
    /// [`MediaEntry::Id`] entries for per-media alt text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_ids: Option<MediaRefs>,
    /// New public media URLs, either shared or a per-platform map. Use
    /// [`MediaEntry::Url`] entries for per-media alt text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_urls: Option<MediaRefs>,
    /// Post type, e.g. `"post"`, `"story"`, `"reel"`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Instagram location tag (a Facebook Place id; see `locations().search`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_id: Option<String>,
    /// Instagram collaborator usernames.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collaborators: Option<Vec<String>>,
    /// Instagram user tags positioned on the image(s).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_tags: Option<Vec<UserTag>>,
    /// Pinterest options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinterest: Option<Value>,
    /// YouTube options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub youtube: Option<Value>,
    /// Instagram options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instagram: Option<Value>,
    /// Facebook options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facebook: Option<Value>,
    /// LinkedIn personal-profile options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin: Option<Value>,
    /// LinkedIn company-page options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_page: Option<Value>,
    /// TikTok options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiktok: Option<Value>,
    /// `json!({"thread_parts": null})` clears thread mode; omit to leave it untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<Value>,
    /// `json!({"thread_parts": null})` clears thread mode; omit to leave it untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bluesky: Option<Value>,
    /// `json!({"thread_parts": null})` clears thread mode; omit to leave it untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mastodon: Option<Value>,
    /// Google Business options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_business: Option<Value>,
}

/// Query for `GET /posts`.
#[derive(Debug, Clone, Default)]
pub struct ListPostsParams {
    /// Filter by status, e.g. `"draft"`, `"scheduled"`, `"published"`, `"failed"`.
    pub status: Option<String>,
    /// Max items to return (default 20, max 100).
    pub limit: Option<u32>,
    /// Items to skip (default 0).
    pub offset: Option<u32>,
}

/// Query for `GET /posts/recent-platform`.
#[derive(Debug, Clone, Default)]
pub struct RecentPlatformPostsParams {
    /// Max posts per platform.
    pub limit: Option<u32>,
    /// Platforms to fetch, e.g. `["instagram", "x"]`. Omit for all connected.
    pub platforms: Option<Vec<String>>,
}

// ─── Media ───────────────────────────────────────────────────────────────────

/// Query for `GET /media`.
#[derive(Debug, Clone, Default)]
pub struct ListMediaParams {
    /// Max items to return (default 20, max 100).
    pub limit: Option<u32>,
    /// Items to skip (default 0).
    pub offset: Option<u32>,
    /// Free-text search over name + filename.
    pub search: Option<String>,
    /// Filter by folder. Use `"root"` (or `"null"`) for unfiled items.
    pub folder_id: Option<String>,
}

/// Multipart body for `POST /media/upload` (max 50MB for images, 100MB
/// request cap overall; use `upload_from_url` or `create_upload_url` for
/// larger videos, up to 1GB).
#[derive(Debug, Clone, Default)]
pub struct UploadMediaParams {
    /// The raw file bytes.
    pub file: Vec<u8>,
    /// Filename sent with the multipart part (the extension matters).
    pub filename: String,
    /// Optional human-readable label so the asset is findable by name later.
    pub name: Option<String>,
    /// Folder name to file the asset under (created at top level if missing).
    pub folder: Option<String>,
    /// Id of an existing folder to file the asset under.
    pub folder_id: Option<String>,
}

/// Body for `POST /media/upload-from-url`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UploadMediaFromUrlParams {
    /// Publicly accessible http(s) URL. Files up to 1GB are supported.
    pub url: String,
    /// Filename override (the extension matters); inferred from the URL when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Optional human-readable label so the asset is findable by name later.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Folder name to file the asset under (created at top level if missing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// Id of an existing folder to file the asset under.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
}

/// Body for `POST /media/upload-from-base64`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UploadMediaFromBase64Params {
    /// Base64-encoded file data (without a data URI prefix).
    pub data: String,
    /// MIME type, e.g. `"image/jpeg"`, `"image/png"`, `"application/pdf"`.
    pub mime_type: String,
    /// Filename to store (the extension matters); derived from the MIME type when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Optional human-readable label so the asset is findable by name later.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Folder name to file the asset under (created at top level if missing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// Id of an existing folder to file the asset under.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
}

/// Body for `POST /media/check`. Provide one of: a public `url`, an existing
/// `media_id`, or `size_bytes` + `mime`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CheckMediaParams {
    /// Public URL of the file to preflight.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Id of an already-uploaded Library item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_id: Option<String>,
    /// Raw size in bytes (pair with `mime`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// MIME type (pair with `size_bytes`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
}

/// Body for `PATCH /media/:id`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateMediaParams {
    /// Human-readable label. Send an empty string to clear it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `Some(Some(id))` moves into that folder, `Some(None)` sends JSON
    /// `null` (move to the root, "All media"), `None` leaves it untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<Option<String>>,
}

// ─── Folders ─────────────────────────────────────────────────────────────────

/// Body for `POST /folders`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateFolderParams {
    /// Folder name.
    pub name: String,
    /// Parent folder id, for a nested folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

/// Body for `PATCH /folders/:id`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateFolderParams {
    /// New folder name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `Some(Some(id))` moves under that folder, `Some(None)` sends JSON
    /// `null` (move to the top level), `None` leaves it untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Option<String>>,
}

// ─── Hashtag sets ────────────────────────────────────────────────────────────

/// The tags of a hashtag set: a list of tags, or one string of tags.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Hashtags {
    /// Individual tags.
    List(Vec<String>),
    /// One string of tags, e.g. `"#a #b"` or `"a, b"`.
    Text(String),
}

impl Default for Hashtags {
    fn default() -> Self {
        Hashtags::List(Vec::new())
    }
}

impl From<Vec<String>> for Hashtags {
    fn from(value: Vec<String>) -> Self {
        Hashtags::List(value)
    }
}

impl From<Vec<&str>> for Hashtags {
    fn from(value: Vec<&str>) -> Self {
        Hashtags::List(value.into_iter().map(str::to_owned).collect())
    }
}

impl From<&str> for Hashtags {
    fn from(value: &str) -> Self {
        Hashtags::Text(value.to_owned())
    }
}

impl From<String> for Hashtags {
    fn from(value: String) -> Self {
        Hashtags::Text(value)
    }
}

/// Body for `POST /hashtag-sets`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateHashtagSetParams {
    /// Set name; posts match it case-insensitively via
    /// [`CreatePostParams::hashtag_set`].
    pub name: String,
    /// The tags: a list, or one string of tags.
    pub hashtags: Hashtags,
}

/// Body for `PATCH /hashtag-sets/:id`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateHashtagSetParams {
    /// New set name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Replaces the FULL hashtag list: a list, or one string of tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashtags: Option<Hashtags>,
}

// ─── Analytics ───────────────────────────────────────────────────────────────

/// Query for `GET /analytics/overview`.
#[derive(Debug, Clone, Default)]
pub struct AnalyticsOverviewParams {
    /// Named period, e.g. `"7d"`, `"30d"`, `"90d"`.
    pub period: Option<String>,
    /// ISO date (YYYY-MM-DD); use with `end_date` instead of `period`.
    pub start_date: Option<String>,
    /// ISO date (YYYY-MM-DD); use with `start_date` instead of `period`.
    pub end_date: Option<String>,
}

/// Query for `GET /analytics/accounts`.
#[derive(Debug, Clone, Default)]
pub struct AccountAnalyticsParams {
    /// Restrict to one platform, e.g. `"instagram"`.
    pub platform: Option<String>,
    /// ISO date (YYYY-MM-DD) to fetch a specific day's snapshot.
    pub date: Option<String>,
}

/// Query for `GET /analytics/best-times`.
#[derive(Debug, Clone, Default)]
pub struct BestTimesParams {
    /// Platform identifier, e.g. `"instagram"`. Required.
    pub platform: String,
    /// IANA timezone for the returned slots, e.g. `"Europe/Amsterdam"`.
    pub timezone: Option<String>,
}

// ─── Inbox ───────────────────────────────────────────────────────────────────

/// Query for `GET /inbox/conversations`.
///
/// The endpoint is cursor-paginated: read `pagination.next_cursor` from the
/// response and pass it back as [`cursor`](ListConversationsParams::cursor) to
/// fetch the next page (stop when `pagination.has_more` is `false`).
#[derive(Debug, Clone, Default)]
pub struct ListConversationsParams {
    /// Restrict to one platform: `"instagram"`, `"facebook"`, or `"linkedin"`.
    pub platform: Option<String>,
    /// Conversation type: `"dm"`, `"comment"`, or `"mention"`.
    pub r#type: Option<String>,
    /// When `true`, only return conversations that have unread messages.
    pub unread: Option<bool>,
    /// Max conversations to return (1-100).
    pub limit: Option<u32>,
    /// Opaque cursor from a previous page's `pagination.next_cursor`.
    pub cursor: Option<String>,
}

/// Query for `GET /inbox/conversations/:id/messages`.
///
/// Cursor-paginated with the same `next_cursor` / `has_more` / `limit`
/// pagination shape as [`ListConversationsParams`].
#[derive(Debug, Clone, Default)]
pub struct GetMessagesParams {
    /// Max messages to return.
    pub limit: Option<u32>,
    /// Opaque cursor from a previous page's `pagination.next_cursor`.
    pub cursor: Option<String>,
}

/// Body for `POST /inbox/conversations/:id/reply`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ReplyParams {
    /// The reply text. Required.
    pub text: String,
    /// Public URL of a single attachment to send with the reply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_url: Option<String>,
    /// Attachment kind: `"image"`, `"video"`, `"audio"`, or `"file"`. Pair
    /// with `attachment_url`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_type: Option<String>,
}

// ─── Webhooks ────────────────────────────────────────────────────────────────

/// Body for `POST /webhooks`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateWebhookParams {
    /// HTTPS endpoint that will receive event deliveries.
    pub url: String,
    /// Events to subscribe to: `post.scheduled`, `post.published`, `post.failed`.
    pub events: Vec<String>,
}

/// Body for `PATCH /webhooks/:id`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateWebhookParams {
    /// New HTTPS endpoint URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// New event subscription list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<String>>,
    /// Pause (`false`) or resume (`true`) deliveries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}
