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

use crate::tag_params::TagRef;

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
/// Bluesky (embed alt), X (photo/GIF media metadata), Pinterest (pin alt_text
/// fallback), Instagram (images), and LinkedIn (images). Plain and
/// alt-carrying entries can be mixed in the same list.
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
    /// The post's tags: tag ids or names (`vec!["Question".into(),
    /// 14.into()]`). A name matches case-insensitively; a name that does not
    /// exist yet becomes a tag of the workspace. At most 20. Tags never go
    /// to a platform.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<TagRef>>,
    /// Route the post through a saved approval workflow (id from
    /// `client.approval_workflows().list()`). The post is created as
    /// `in_approval` (`approval_status: "pending"`) instead of `scheduled`;
    /// its approvers are notified and it publishes at `scheduled_at` once the
    /// last step approves (dashboard, or `posts().approve`). Requires
    /// `scheduled_at`; not allowed with `publish_now`. Errors:
    /// `404 workflow_not_found`, `400 validation_error`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_workflow_id: Option<String>,
    /// Planned draft. With `scheduled_at`, the post is stored as `draft` and
    /// keeps that time: it shows on the dashboard calendar at that slot with
    /// a dashed outline, the scheduler never publishes it, and a person
    /// schedules it from the composer (or you do with `posts().update` and
    /// `draft: Some(false)`). Use it when an agent proposes a slot that a
    /// human should confirm. Not allowed with `publish_now` or
    /// `approval_workflow_id` (`400 validation_error`). Without
    /// `scheduled_at` it changes nothing: that post is a draft already.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<bool>,
    /// Image fit per channel for feed images whose ratio is outside what a
    /// channel shows uncropped, keyed by platform id (`"instagram"`,
    /// `"pinterest"`, ...), each `"auto"`, `"crop"` or `"keep"`. `auto`
    /// (default): Instagram crops a tall or wide image to 4:5 / 1.91:1
    /// around the subject (Instagram cuts it to that window anyway and Meta
    /// cannot boost a feed post outside it); Pinterest keeps a Pin taller
    /// than 2:3 and reports it. `crop` / `keep` force it per channel.
    /// Decisions come back in the response's `warnings` with code
    /// `image_fit`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_fit: Option<HashMap<String, String>>,
    /// Comment-to-DM automation bound to this post (Instagram or Facebook
    /// must be in `channels`): a keyword in a comment is answered with a DM,
    /// once per person, with an optional public reply under the comment.
    /// E.g. `json!({"trigger": {"keywords": ["guide"], "match": "word"},
    /// "actions": {"dm": {"text": "..."}, "public_reply": {"enabled": true,
    /// "variations": ["..."]}}, "limits": {"max_sends": 500}, "platforms":
    /// ["instagram", "facebook"]})` (see [`CreateAutomationParams`] for the
    /// fields, also `actions.follow_gate`, Follow first, which applies to the
    /// Instagram automation only). `platforms` makes one automation per
    /// listed channel; each
    /// must be a channel of the post. Without it: Instagram when the post
    /// goes there, else Facebook. Needs the `automations:write` scope on the
    /// key (`403 insufficient_scope`, checked before anything else); a bad
    /// object is a `400 validation_error` whose message starts with
    /// `comment_dm_automation: `. The response then carries
    /// `data.comment_dm_automation` (the first automation, with `post`
    /// null, plus `platforms` and `automations`, one `id` / `platform` /
    /// `status` per channel); each starts listening once the post is live
    /// on its channel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_dm_automation: Option<Value>,
    /// Pinterest options (e.g. `json!({"board_id": "...", "title": "..."})`).
    /// `product_tags` tags products on the Pin: up to 24 product Pins of the
    /// connected Pinterest account, each as a Pin id string (see
    /// `client.pinterest().list_products`) or a Pin link. Products of other
    /// merchants cannot be tagged. The tags are added right after the Pin is
    /// published; a product Pinterest refuses never fails the post, and the
    /// post's `pinterest` block then carries `product_tags_result`
    /// (`requested`, `tagged`, `skipped`, `error`). More than 24 entries or
    /// an invalid entry returns `400 validation_error`.
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
    /// max 280 chars — 25,000 for Premium/Premium+ accounts — and optional
    /// `media_ids`/`media_urls`, max 4) to publish a thread; also supports
    /// `reply_settings`, `paid_partnership`, `made_with_ai`. Thread-part
    /// media entries accept `{"url"|"id": ..., "alt": ...}` objects for
    /// per-media alt text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<Value>,
    /// Bluesky options. Same `thread_parts` shape as `x` (300 chars per part).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bluesky: Option<Value>,
    /// Mastodon options. Same `thread_parts` shape as `x` (500 chars per part).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mastodon: Option<Value>,
    /// Threads (Meta) options. Same `thread_parts` shape as `x` (2-25 parts,
    /// 500 chars per part, up to 10 media per part; parts after the first
    /// publish as replies to the previous part, and the Threads caption is
    /// taken from part 1). Also accepts `location_id`: a Threads location id
    /// from [`crate::resources::Locations::search_with`] with platform
    /// `"threads"` (on a multi-part thread the tag is applied to part 1), or
    /// alternatively `location`: an object `{id, name?, address?, city?,
    /// country?}` to store display fields along with the id (`location_id`
    /// wins when both are given). Threads location tagging is currently
    /// rolling out; until Meta approves the permissions it is disabled on
    /// production and calls return a clear error. `topic_tag` sets the topic
    /// of the post, for example `"Build in public"` (1 to 50 characters, no
    /// period and no ampersand; on a multi-part thread it goes on part 1;
    /// `null` on update removes it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<Value>,
    /// Google Business options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_business: Option<Value>,
    /// Telegram options (the connected channel or group, posted by the
    /// OmniSocials bot), e.g. `json!({"silent": true, "pin": true,
    /// "buttons": [{"text": "Shop", "url": "https://..."}]})`. Keys:
    /// `silent`, `link_preview`, `pin`, `protect_content`, `spoiler`,
    /// `caption_above_media`, up to 3 `buttons` (not on an album or a poll),
    /// a native `poll` (`{"options": [...], "open_period": 3600, "multiple":
    /// true}`: 2 to 12 options of up to 100 chars; the post text is the
    /// question, max 300 chars, no media) and `first_comment`. Text up to
    /// 4,096 chars; with media, text over 1,024 chars is sent as a reply
    /// under the media. No Stories.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram: Option<Value>,
    /// Discord options (one Discord channel, posted through its incoming
    /// webhook with the webhook's name and picture), e.g.
    /// `json!({"silent": true, "buttons": [{"text": "Notes", "url":
    /// "https://..."}]})`. Keys: `silent`, `link_preview` (default on),
    /// `allow_mentions` (default off: mentions show as text and ping nobody),
    /// `spoiler`, `thread_name` (the post title in a forum or media channel,
    /// 1 to 100 chars), up to 5 `buttons` (1 to 80 chars, https:// or
    /// http://, one row, not with a poll) and a native `poll`
    /// (`{"options": [...], "duration_hours": 24, "multiple": true}`: 2 to 10
    /// options of up to 55 chars, 1 to 768 hours; the post text is the
    /// question, max 300 chars, no media and no buttons). Text up to 2,000
    /// chars; up to 10 images, GIFs and videos per message, a video or GIF
    /// over 20 MB is sent as a link in the text. No Stories.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discord: Option<Value>,
    /// Non-sponsored LinkedIn poll (e.g. `json!({"question": "...",
    /// "options": ["A", "B"], "duration": "ONE_DAY"})`). Question max 140
    /// chars; 2-4 `options`, each max 30 chars; `duration` one of
    /// `"ONE_DAY"`, `"THREE_DAYS"`, `"SEVEN_DAYS"`, `"FOURTEEN_DAYS"`.
    /// Mutually exclusive with media and a link share on the same post.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_poll: Option<Value>,
    /// Video thumbnail for a post whose media is one video, e.g.
    /// `json!({"type": "frame", "thumb_offset": 3000})` or
    /// `json!({"type": "custom", "cover_url": "https://..."})`, plus an
    /// optional `"overrides"` map keyed by platform id. Applied on
    /// Instagram, Facebook, LinkedIn, TikTok (frame only), Pinterest and
    /// YouTube Shorts (shown on Shorts only on channels where YouTube has
    /// enabled custom Shorts thumbnails).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_cover: Option<Value>,
}

/// Body for `PATCH /posts/:id`.
///
/// For `x`, `bluesky`, `mastodon`, and `threads`: passing `"thread_parts": null` inside
/// the value explicitly clears thread mode (reverts to a single post), while
/// omitting the field leaves the existing thread untouched.
///
/// For `linkedin_poll`: passing `json!(null)` as the field's own value
/// explicitly clears the poll (reverts to a normal post), while omitting the
/// field leaves the existing poll untouched.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdatePostParams {
    /// New caption. Either a single string or a per-platform map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    /// New ISO 8601 schedule time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<String>,
    /// Planned-draft switch. `Some(false)` without `scheduled_at` schedules
    /// a draft at the time it already carries (`400 validation_error` when
    /// it has none or it is in the past). `Some(true)` without
    /// `scheduled_at` unschedules a scheduled post: it keeps its time and
    /// stays on the calendar as a dashed draft, nothing publishes; a post
    /// waiting for approval leaves its workflow. `Some(true)` with
    /// `scheduled_at` moves the time and keeps (or makes) the post a draft.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<bool>,
    /// Image fit per channel (same shape as [`CreatePostParams::media_fit`]).
    /// On update, `media_fit` alone re-decides the stored images (`keep`
    /// gives the original back). Decisions come back in the response's
    /// `warnings` with code `image_fit`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_fit: Option<HashMap<String, String>>,
    /// Replaces the post's comment-to-DM automations (same shape as
    /// [`CreatePostParams::comment_dm_automation`]; a channel no longer in
    /// `platforms` loses its automation). `json!(null)` removes them all;
    /// omit to leave them untouched. Needs the `automations:write` scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_dm_automation: Option<Value>,
    /// Replaces the post's tags (ids or names, a new name becomes a tag).
    /// `Some(None)` sends JSON `null` and `Some(Some(vec![]))` sends `[]`:
    /// both remove every tag. `None` leaves them untouched. An update that
    /// sets only `tags` works on every post, published ones included.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Option<Vec<TagRef>>>,
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
    /// Pinterest options. Replaces the stored Pinterest options wholesale,
    /// so leave `product_tags` out (or send `[]`) to remove the product tags.
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
    /// `json!({"thread_parts": null})` clears thread mode,
    /// `json!({"location_id": null})` (or `json!({"location": null})`) clears
    /// the Threads location tag and `json!({"topic_tag": null})` removes the
    /// topic; omit a field to leave it untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<Value>,
    /// Google Business options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_business: Option<Value>,
    /// Telegram options (see [`CreatePostParams::telegram`] for the keys).
    /// Every key merges into the stored options; a key set to `null` removes
    /// it (e.g. `json!({"poll": null})`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram: Option<Value>,
    /// Discord options (see [`CreatePostParams::discord`] for the keys).
    /// Every key merges into the stored options; a key set to `null` removes
    /// it (e.g. `json!({"poll": null})`), and `Some(json!(null))` removes
    /// every Discord option. Omit to leave them untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discord: Option<Value>,
    /// Non-sponsored LinkedIn poll (see [`CreatePostParams::linkedin_poll`]
    /// for the shape). `Some(Value::Null)` (i.e. `json!(null)`) explicitly
    /// clears the poll, reverting the post to normal; omitting the field
    /// leaves the existing poll untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_poll: Option<Value>,
    /// Replaces the stored video cover wholesale (see
    /// [`CreatePostParams::video_cover`]). `json!(null)` removes it; omit to
    /// leave it untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_cover: Option<Value>,
}

/// Query for `GET /posts`.
#[derive(Debug, Clone, Default)]
pub struct ListPostsParams {
    /// Filter by status: `"draft"`, `"in_approval"`, `"scheduled"`, `"posting"`,
    /// `"published"`, `"failed"`, `"warning"`. `"in_approval"` = waiting for a
    /// reviewer in an approval workflow.
    pub status: Option<String>,
    /// Max items to return (default 20, max 100).
    pub limit: Option<u32>,
    /// Items to skip (default 0).
    pub offset: Option<u32>,
    /// Only posts whose `schedule_at` is on or after this moment (for a post
    /// sent at once: when it was sent). `"YYYY-MM-DD"` (a UTC day),
    /// `"YYYY-MM"` (a month) or an ISO 8601 datetime. With a range the posts
    /// come in calendar order (earliest first).
    pub start_date: Option<String>,
    /// Only posts whose `schedule_at` is on or before this moment
    /// (inclusive). Same formats as `start_date`.
    pub end_date: Option<String>,
    /// Only posts with this tag: a tag id or name. Comma separate several
    /// to match posts with any of them. An unknown tag answers 404
    /// `tag_not_found`.
    pub tag: Option<String>,
}

/// Body for `POST /posts/:id/approve`
/// (see [`crate::resources::Posts::approve_with`]).
#[derive(Debug, Clone, Default, Serialize)]
pub struct ApprovePostParams {
    /// The workflow step you reviewed: `current_step` of
    /// [`crate::resources::Posts::get_approval`]. When the review is on
    /// another step now, nothing is approved and the answer is `409
    /// step_changed` with `error.current_step_order`. A whole number from 1,
    /// else `400 validation_error`. `None` acts on whatever step is current.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_order: Option<u32>,
}

/// Body for `POST /posts/:id/reject`
/// (see [`crate::resources::Posts::reject_with`]).
#[derive(Debug, Clone, Default, Serialize)]
pub struct RejectPostParams {
    /// Reason shown in the post's review thread (max 2000 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// The workflow step you reviewed, as on [`ApprovePostParams`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_order: Option<u32>,
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
    /// PDF uploads only: `"slides"` (default, one image item per page) or
    /// `"document"` (one item whose single id in `media_ids` expands into
    /// every page).
    pub pdf_mode: Option<String>,
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
    /// PDF uploads only: `"slides"` (default) or `"document"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_mode: Option<String>,
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
    /// PDF uploads only: `"slides"` (default) or `"document"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_mode: Option<String>,
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
    /// Only the post numbers of the posts with this tag: a tag id or name,
    /// comma separated for any of several. Account numbers stay those of
    /// the whole account.
    pub tag: Option<String>,
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

// ─── Locations ───────────────────────────────────────────────────────────────

/// Query for `GET /locations/search`
/// (see [`crate::resources::Locations::search_with`]).
///
/// Pass either `q` or the `latitude` + `longitude` pair (coordinates are
/// Threads only); neither, both, a `q` under 2 characters, or out-of-range
/// coordinates return a 400 validation error.
#[derive(Debug, Clone, Default)]
pub struct SearchLocationsParams {
    /// Free-text search (min 2 characters).
    pub q: Option<String>,
    /// Location source: `"instagram"` (Facebook Places, the default) or
    /// `"threads"` (Meta's Threads location catalog). The two sources use
    /// different ids.
    pub platform: Option<String>,
    /// Latitude (-90..90). Threads only; pair with `longitude` instead of `q`.
    pub latitude: Option<f64>,
    /// Longitude (-180..180). Threads only; pair with `latitude` instead of `q`.
    pub longitude: Option<f64>,
}

// ─── Pinterest ───────────────────────────────────────────────────────────────

/// Query for `GET /pinterest/products`
/// (see [`crate::resources::Pinterest::list_products`]). Every field is
/// optional.
#[derive(Debug, Clone, Default)]
pub struct ListPinterestProductsParams {
    /// Where to read product Pins from: `"catalog"` (the Pinterest catalog,
    /// needs catalog access) or `"pins"` (the account's own Pins). Default:
    /// `"catalog"` when the connection has catalog access, else `"pins"`.
    pub source: Option<String>,
    /// Catalog source only. A product group `id` from `product_groups`.
    /// Default: the group named "All Products", else the first group.
    pub product_group_id: Option<String>,
    /// Cursor from the previous response's `bookmark`, to get the next page.
    pub bookmark: Option<String>,
    /// Catalog source only. Products per page (1..100, default 25).
    pub page_size: Option<u32>,
}

// ─── Automations (comment-to-DM) ─────────────────────────────────────────────

/// Query for `GET /automations`
/// (see [`crate::resources::Automations::list`]). Every field is optional.
#[derive(Debug, Clone, Default)]
pub struct ListAutomationsParams {
    /// `"active"`, `"paused"` or `"completed"`.
    pub status: Option<String>,
    /// `"instagram"`, `"facebook"` or `"x"`: only that channel's
    /// automations.
    pub platform: Option<String>,
    /// Only the automations bound to this OmniSocials post.
    pub post_id: Option<String>,
}

/// Query for `GET /automations/:id/events`
/// (see [`crate::resources::Automations::list_events`]). Every field is
/// optional.
#[derive(Debug, Clone, Default)]
pub struct ListAutomationEventsParams {
    /// `"sent"`, `"skipped"`, `"failed"`, `"pending"`, `"queued"` (the
    /// answer waits for a free slot under the send cap and goes out within
    /// the hour), `"opted_out"` (the person replied STOP on X) or
    /// `"waiting"` (Follow first: asked to follow, the link goes out after
    /// the follow).
    pub outcome: Option<String>,
    /// Events per page, 1 to 100 (default 25).
    pub limit: Option<u32>,
    /// Events to skip.
    pub offset: Option<u32>,
}

/// Body for `POST /automations`
/// (see [`crate::resources::Automations::create`]).
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateAutomationParams {
    /// `"instagram"` (default), `"facebook"` (a Facebook Page; the DM lands
    /// in Messenger) or `"x"` (the X account: `trigger.kinds` `["dm"]` only,
    /// no post fields). `facebook_post_id` implies `"facebook"`. Facebook
    /// has no story replies: `story_reply` in `trigger.kinds` is a
    /// `400 validation_error`. An active X automation needs X DMs on for the
    /// workspace (`409 x_dms_off`; `status` `"paused"` always saves), and
    /// every X answer ends with "Reply STOP to stop automatic messages.".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// Name, max 120 characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// For scope `post`: an OmniSocials post that goes to the automation's
    /// channel (scheduled or live). One automation per post and platform: a
    /// post that goes to Instagram and Facebook can have one of each. Set at
    /// most one of `post_id`, `instagram_media_id` and `facebook_post_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_id: Option<String>,
    /// For scope `post`: an Instagram post published outside OmniSocials
    /// (numeric Instagram media id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instagram_media_id: Option<String>,
    /// For scope `post`: a post published on the Facebook Page outside
    /// OmniSocials, as `{page-id}_{post-id}` or the post id alone (the Page
    /// id is put in front). Implies `platform` `"facebook"`. A post of
    /// another Page is a `404 facebook_post_not_found`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facebook_post_id: Option<String>,
    /// `json!({"kinds": ["comment"], "scope": "post", "keywords": ["guide"],
    /// "match": "word"})`: `kinds` one or more of `comment`, `dm`,
    /// `story_reply` (default `["comment"]`; a Facebook Page has `comment`
    /// and `dm` only, X has `dm` only); `scope` `post` (default), `next_post` (the next post
    /// the workspace publishes on the automation's channel) or `any` (every
    /// post); `keywords` up to 20 of 60 characters, required
    /// unless `match` is `any`; `match` `word` (default), `contains` or
    /// `any`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<Value>,
    /// Required: `json!({"dm": {"text": "..."}})` (max 1000 characters) plus
    /// an optional `"public_reply": {"enabled": true, "variations": [...]}`
    /// (comment kind only, up to 10 variations of 300 characters) and an
    /// optional `"follow_gate": {"enabled": true, "first_message": {"text":
    /// "...", "button": "..."}, "not_following": {"text": "...", "button":
    /// "..."}}`: Follow first, Instagram only. The link (the `dm` text) then
    /// goes out only to people who follow the account: a comment gets the
    /// first message as the private reply, with one button, and a tap checks
    /// the follow; anyone who does not follow yet gets the `not_following`
    /// message with its button, and every later tap checks again. A DM or a
    /// story reply is checked at once. When Instagram gives no answer the
    /// link goes out anyway. Texts hold 640 characters, buttons 20; a text
    /// or button left out is the default. `"enabled": true` on a Facebook or
    /// X automation is a `400 follow_gate_instagram_only`. The response
    /// carries the gate with its texts (null on Facebook and X) and
    /// `stats.followed`.
    pub actions: Value,
    /// `json!({"max_sends": 500, "stop_at": "2026-12-31T00:00:00Z"})`, both
    /// optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<Value>,
    /// `"active"` (default) or `"paused"` (create as paused).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Body for `PATCH /automations/:id`
/// (see [`crate::resources::Automations::update`]). Every field is
/// optional; a body with nothing is a `400 no_fields`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateAutomationParams {
    /// The platform cannot change: another platform (or
    /// `instagram_media_id` on a Facebook automation, `facebook_post_id` on
    /// an Instagram one) is a `400 platform_immutable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// New name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Moves the automation to that OmniSocials post.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_id: Option<String>,
    /// Moves an Instagram automation to that Instagram post.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instagram_media_id: Option<String>,
    /// Moves a Facebook automation to that Facebook Page post.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facebook_post_id: Option<String>,
    /// Merged into the stored trigger (see
    /// [`CreateAutomationParams::trigger`]): keys you omit keep their value,
    /// so new `keywords` keep the kinds, the scope and the match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<Value>,
    /// Merged into the stored actions (see
    /// [`CreateAutomationParams::actions`]): a new `dm.text` keeps the
    /// public reply; `"public_reply": {"enabled": false}` or
    /// `{"variations": []}` turns it off. `follow_gate` merges too:
    /// `{"enabled": false}` keeps its texts, new texts keep the switch,
    /// `null` turns it off.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Value>,
    /// Merged into the stored limits (see
    /// [`CreateAutomationParams::limits`]): `json!({"stop_at": "..."})`
    /// keeps `max_sends`; `json!({"max_sends": null})` removes that limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<Value>,
    /// `"paused"` pauses, `"active"` resumes (an X automation only while X
    /// DMs are on for the workspace, else `409 x_dms_off`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

// ─── Ads ─────────────────────────────────────────────────────────────────────
//
// Ads spend real money once a campaign starts. Boosts and drafts are created
// as drafts (no spend). Starting or raising spend needs a key with the opt-in
// `ads:spend` scope ("Start ads", a second switch under "Ads"; its creator
// needs `manage_ads`): `activate`, `activate_multi`, `resume`,
// `drafts().launch`, a budget raise or any change of a campaign that reached
// the network through `update`, and the parity calls that switch something
// on or raise a budget. Each repeats the amount. Without `ads:spend` these
// calls answer `403 start_in_app` with the page in the app where a person
// does it; on Google (`api_draft_only`) they always do, also with
// `ads:spend`. Pause and delete stay open, except on the networks in
// `api_stop_in_app` (Google). Budgets are in MAJOR currency units (10 = 10
// EUR) in the ad account's currency. Ids are strings, like every other id in
// this crate; the API reads `"42"` and `42` the same.

/// Query for `GET /ads/campaigns`
/// (see [`crate::resources::Ads::list`]). Every field is optional.
#[derive(Debug, Clone, Default)]
pub struct ListAdCampaignsParams {
    /// `"draft"`, `"pending_review"`, `"active"`, `"rejected"`, `"paused"`,
    /// `"completed"` or `"error"`.
    pub status: Option<String>,
    /// `"meta"`, `"pinterest"`, `"tiktok"`, `"linkedin"`, `"x"` or
    /// `"google"`.
    pub platform: Option<String>,
    /// `"campaign"` or `"boost"`.
    pub campaign_type: Option<String>,
    /// Campaigns per page, 1 to 100 (default 25).
    pub limit: Option<u32>,
    /// Campaigns to skip.
    pub offset: Option<u32>,
}

/// A campaign budget, in MAJOR currency units (10 = 10 EUR). Network
/// minimums apply (for example TikTok 20 a day).
#[derive(Debug, Clone, Default, Serialize)]
pub struct AdBudget {
    /// `"daily"` (default) or `"lifetime"` (the whole schedule). A draft
    /// budget is always daily: leave it `None` there.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// The amount in major units.
    pub amount: f64,
    /// 3-letter ISO code; must match the ad account's currency.
    pub currency: String,
}

/// When a campaign runs (ISO 8601 datetimes).
#[derive(Debug, Clone, Default, Serialize)]
pub struct AdSchedule {
    /// Start time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_at: Option<String>,
    /// End time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_at: Option<String>,
}

/// Body for `POST /ads/boost` (see [`crate::resources::Ads::boost`]).
#[derive(Debug, Clone, Default, Serialize)]
pub struct BoostPostParams {
    /// OmniSocials id of a published post.
    pub post_id: String,
    /// OmniSocials ad account id (from `list_accounts`, picked in the
    /// dashboard).
    pub ad_account_id: String,
    /// The budget; its currency must match the ad account's.
    pub budget: AdBudget,
    /// `"engagement"`, `"traffic"`, `"awareness"` or `"video_views"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objective: Option<String>,
    /// Unified targeting, e.g. `json!({"countries": ["NL"]})`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targeting: Option<Value>,
    /// When the boost runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<AdSchedule>,
    /// Network-specific creative fields. TikTok (Spark Ads) needs
    /// `spark_auth_code` (the post owner makes it in the TikTok app: video,
    /// "...", Ad settings, Ad authorization); Google needs `headlines`,
    /// `descriptions` and `logo_url`; TikTok traffic boosts need `link`.
    /// Meta, Pinterest, X and LinkedIn boosts need none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creative: Option<Value>,
    /// Campaign name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Body for `POST /ads/boost-multi` (see
/// [`crate::resources::Ads::boost_multi`]): one draft boost per ad account,
/// each with the SAME budget.
#[derive(Debug, Clone, Default, Serialize)]
pub struct BoostPostMultiParams {
    /// OmniSocials id of a published post.
    pub post_id: String,
    /// Up to 10 ad accounts.
    pub ad_account_ids: Vec<String>,
    /// The budget of each boost.
    pub budget: AdBudget,
    /// See [`BoostPostParams::objective`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objective: Option<String>,
    /// See [`BoostPostParams::targeting`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targeting: Option<Value>,
    /// See [`BoostPostParams::schedule`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<AdSchedule>,
    /// See [`BoostPostParams::creative`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creative: Option<Value>,
    /// Campaign name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Body for `PATCH /ads/campaigns/:id` (see
/// [`crate::resources::Ads::update`]). Every field is optional. A draft
/// changes with `ads:write`. A campaign that reached the network changes
/// only with a key that has `ads:spend` (never on Google); without it the
/// call answers `403 start_in_app`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateAdCampaignParams {
    /// New name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A new goal, only on a draft: `"traffic"`, `"engagement"`,
    /// `"awareness"`, `"video_views"`, or through the API only
    /// `"lead_generation"`, `"conversions"`, `"messages"` or `"calls"`
    /// (`capabilities` says per network what each `needs`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objective: Option<String>,
    /// New budget.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<AdBudget>,
    /// Required to RAISE the budget of a running campaign (a key with
    /// `ads:spend`, outside `api_draft_only`): the new amount again, after
    /// the person agreed to it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_budget_amount: Option<f64>,
    /// New targeting: unified targeting (`countries`, ages, genders) plus
    /// `network`, the network's own targeting (it may change after the start
    /// too): `json!({"countries": ["NL"], "network": {"include":
    /// {"interest": [{"id": "6003", "name": "Yoga"}]}, "exclude": {},
    /// "audiences": {"include": [], "exclude": []}}})`, ids from
    /// `ads().targeting().search`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targeting: Option<Value>,
    /// New schedule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<AdSchedule>,
    /// New settings, only on a campaign that never started (after the start
    /// `400 settings_locked`: make a copy with
    /// [`crate::resources::Ads::duplicate`]): `optimization` (`conversion`,
    /// `bidding`, `attribution`), `messaging`, `calls`, `lead_form_id`,
    /// `google` (`kind`, `search`, `performance_max`) and `meta`
    /// (`budget_level`). `capabilities` lists the keys per network.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Value>,
}

/// Body for `POST /ads/research/keywords` (see
/// [`crate::resources::Ads::research_keywords`]). Give `keywords`, `url` or
/// both.
#[derive(Debug, Clone, Default, Serialize)]
pub struct KeywordResearchParams {
    /// Seed keywords, up to 20.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    /// A landing page or homepage to take ideas from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// ISO 3166-1 alpha-2, default `"US"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// ISO 639-1, default `"en"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// 1 to 500, default 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Only needed when the workspace has more than one Google Ads account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ad_account_id: Option<String>,
}

/// Body for `POST /ads/research/keywords/metrics` (see
/// [`crate::resources::Ads::keyword_metrics`]).
#[derive(Debug, Clone, Default, Serialize)]
pub struct KeywordMetricsParams {
    /// The exact keywords to measure.
    pub keywords: Vec<String>,
    /// ISO 3166-1 alpha-2, default `"US"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// ISO 639-1, default `"en"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Only needed when the workspace has more than one Google Ads account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ad_account_id: Option<String>,
}

/// One network of [`AdDraftParams::platforms`]: a network key, or a key with
/// its own daily amount or ad account.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum AdDraftPlatform {
    /// A network key: `"meta"`, `"pinterest"`, `"tiktok"`, `"linkedin"`,
    /// `"x"` or `"google"`.
    Key(String),
    /// A network with its own daily amount or ad account.
    Detailed {
        /// The network key.
        platform: String,
        /// The daily amount on this network, in major units.
        #[serde(skip_serializing_if = "Option::is_none")]
        budget_amount: Option<f64>,
        /// The ad account on this network.
        #[serde(skip_serializing_if = "Option::is_none")]
        ad_account_id: Option<String>,
        /// The settings of this network (see
        /// [`UpdateAdCampaignParams::settings`]); `None` keeps the stored
        /// settings on an update.
        #[serde(skip_serializing_if = "Option::is_none")]
        settings: Option<Value>,
    },
}

impl From<&str> for AdDraftPlatform {
    fn from(value: &str) -> Self {
        AdDraftPlatform::Key(value.to_owned())
    }
}

impl From<String> for AdDraftPlatform {
    fn from(value: String) -> Self {
        AdDraftPlatform::Key(value)
    }
}

/// How long a draft runs.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AdDraftSchedule {
    /// Days; `None` is sent as `null` and runs the ad until it is stopped.
    pub duration_days: Option<u32>,
}

/// Who a draft reaches.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AdDraftTargeting {
    /// ISO country codes.
    pub countries: Vec<String>,
    /// `true` (the default) lets the network find the audience (no age or
    /// gender limits).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatic_audience: Option<bool>,
    /// Lowest age.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_min: Option<u32>,
    /// Highest age.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_max: Option<u32>,
    /// `"male"`, `"female"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genders: Option<Vec<String>>,
    /// `"automatic"` (default) or `"feeds"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placements: Option<String>,
    /// LinkedIn's EU statement: `"NOT_POLITICAL"` or `"POLITICAL"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub political_intent: Option<String>,
    /// Meta's special ad categories (housing, employment, financial
    /// products, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special_ad_categories: Option<Vec<String>>,
    /// The countries of the special ad categories.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special_ad_category_country: Option<Vec<String>>,
    /// Meta's verified payer per regional category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regional_identities: Option<Value>,
    /// The network's own targeting per network key, e.g. `json!({"meta":
    /// {"include": {"interest": [{"id": "6003", "name": "Yoga"}]},
    /// "audiences": {"exclude": [{"id": "aud-1"}]}}})`, ids from
    /// `ads().targeting().search`. `location` entries replace `countries` on
    /// that network. `None` keeps each network's stored targeting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<Value>,
}

/// One picture or video of a draft ad.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AdMediaFile {
    /// Public URL.
    pub url: String,
    /// `"image"` or `"video"`.
    #[serde(rename = "type")]
    pub r#type: String,
    /// Width in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Height in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Seconds, for a video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// Cover picture of a video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
}

/// Extra files for one kind of placement.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AdPlacementMedia {
    /// Meta only, optional: a second picture or video for Stories and
    /// Reels, upright 9:16 (1080 x 1920), the same kind as the main file.
    /// Without it Meta fits the main file. Not for the engagement goal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stories_reels: Option<AdMediaFile>,
}

/// One ad of a draft: written once, mapped onto every network.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AdDraftAd {
    /// Your own key for the ad.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Ad name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The pictures or videos.
    pub media: Vec<AdMediaFile>,
    /// The main text.
    pub primary_text: String,
    /// A second text: the network tests A against B (a second ad).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_text_b: Option<String>,
    /// Headline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headline: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// A button key from `capabilities` `ctas`, e.g. `"shop_now"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cta: Option<String>,
    /// The website the ad opens.
    pub link: String,
    /// Per network text that replaces the shared text, e.g.
    /// `json!({"x": {"primary_text": "..."}})`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<Value>,
    /// Network-only fields, e.g. `json!({"google": {"business_name": "...",
    /// "logo": {"url": "..."}}})`. Meta: `lead_form_id`, `primary_texts`,
    /// `headlines`, `creative_features`, `media_ref`, `partnership`; TikTok:
    /// `spark_auth_code`, `lead_form_id`, `media_ref`; LinkedIn and
    /// Pinterest: `lead_form_id`; X: `media_ref`; Google: `business_name`,
    /// `logo`, `responsive_search_ad` (`capabilities` lists them as
    /// `ad_fields`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Value>,
    /// Extra files per placement (Meta Stories and Reels).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement_media: Option<AdPlacementMedia>,
}

/// Body for `POST /ads/drafts` and `PUT /ads/drafts/:group_id` (the full
/// draft; see [`crate::resources::AdDrafts::create`]).
#[derive(Debug, Clone, Default, Serialize)]
pub struct AdDraftParams {
    /// Draft name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `"traffic"`, `"engagement"`, `"awareness"` or `"video_views"`, or
    /// through the API only `"lead_generation"`, `"conversions"`,
    /// `"messages"` or `"calls"` (`capabilities` says per network what each
    /// `needs`: a lead form, a conversion, messaging or a phone number in
    /// the network's `settings`).
    pub objective: String,
    /// The networks: `vec!["meta".into(), "linkedin".into()]`, or
    /// [`AdDraftPlatform::Detailed`] entries for a daily amount or an ad
    /// account per network.
    pub platforms: Vec<AdDraftPlatform>,
    /// The daily amount per network (leave `type` `None`).
    pub budget: AdBudget,
    /// How long it runs; `None` leaves it out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<AdDraftSchedule>,
    /// Who it reaches.
    pub targeting: AdDraftTargeting,
    /// The ads.
    pub ads: Vec<AdDraftAd>,
}

/// Body for `POST /ads/drafts/:group_id/account` (see
/// [`crate::resources::AdDrafts::attach_account`]).
#[derive(Debug, Clone, Default, Serialize)]
pub struct AttachAdDraftAccountParams {
    /// The network key.
    pub platform: String,
    /// OmniSocials ad account id of the same network.
    pub ad_account_id: String,
}

/// Body for `POST /ads/drafts/:group_id/launch` (see
/// [`crate::resources::AdDrafts::launch`]). Give `confirm_budget_amount`
/// (one daily amount for every network) or `confirm_budget_amounts` (per
/// campaign id). Each must equal that campaign's daily amount, else
/// `400 budget_confirmation_required` and nothing is written. Needs a key
/// with `ads:spend`: without it the call answers `403 start_in_app` with the
/// draft's page. A network in `api_draft_only` (Google) is skipped with code
/// `start_in_app`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct LaunchAdDraftParams {
    /// The daily amount of every network, as the person confirmed it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_budget_amount: Option<f64>,
    /// The daily amount per campaign id, as the person confirmed it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_budget_amounts: Option<HashMap<String, f64>>,
}

// ─── Inbox ───────────────────────────────────────────────────────────────────

/// Query for `GET /inbox/conversations`.
///
/// The endpoint is cursor-paginated: read `pagination.next_cursor` from the
/// response and pass it back as [`cursor`](ListConversationsParams::cursor) to
/// fetch the next page (stop when `pagination.has_more` is `false`).
#[derive(Debug, Clone, Default)]
pub struct ListConversationsParams {
    /// Restrict to one platform: `"instagram"`, `"facebook"`, `"linkedin"`,
    /// `"tiktok"`, `"youtube"`, `"x"`, or `"threads"`.
    pub platform: Option<String>,
    /// Conversation type: `"dm"`, `"comment"`, or `"mention"`. Threads has
    /// `"comment"` and `"mention"` only (no DMs).
    pub r#type: Option<String>,
    /// When `true`, only return conversations that have unread messages.
    pub unread: Option<bool>,
    /// When `true`, only return conversations that still need an answer: the
    /// customer's latest DM has no reply after it (Instagram/Facebook DMs
    /// past Meta's 24-hour messaging window included: they cannot be
    /// answered through the API, but the customer is still waiting), or a
    /// comment/mention that has not been replied to and is not hidden.
    /// Replies typed in the native apps count as answers (they are mirrored
    /// into the inbox). Read state is ignored here; use
    /// [`next`](crate::resources::Inbox::next) for a work queue.
    pub unanswered: Option<bool>,
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
    /// The reply text. Optional when `attachment_url` is set — an
    /// attachment-only reply is allowed.
    pub text: String,
    /// Public URL of a single attachment to send with the reply (Facebook
    /// and Instagram DMs only; other platforms are text-only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_url: Option<String>,
    /// Attachment kind: `"image"`, `"video"`, `"audio"`, or `"file"`. Pair
    /// with `attachment_url`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_type: Option<String>,
    /// For comment and mention threads: the inbox id of the specific incoming
    /// comment being answered (`message.id` from
    /// [`next`](crate::resources::Inbox::next), or a message `id` from
    /// [`get_messages`](crate::resources::Inbox::get_messages)). Every
    /// comment on a post shares one conversation, so without it the reply is
    /// posted under the newest comment on the post, which may be a different
    /// person than the one you drafted for. Always set it when replying to
    /// an item served by the queue. Ignored for DMs (a DM reply goes to the
    /// conversation). 404 `not_found` when it is not an incoming message of
    /// this conversation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    /// When `true`, the response also carries `next` (the next conversation
    /// that needs an answer, the same object
    /// [`next`](crate::resources::Inbox::next) returns under `data`, using
    /// its default queue order and filters; `null` when nothing is waiting)
    /// and `remaining`. Saves the extra call when working through the inbox.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_next: Option<bool>,
}

/// Query for `GET /inbox/next`. All fields are optional.
#[derive(Debug, Clone, Default)]
pub struct NextUnansweredParams {
    /// Only items from one platform: `"instagram"`, `"facebook"`,
    /// `"linkedin"`, `"tiktok"`, `"youtube"`, `"x"`, or `"threads"`.
    pub platform: Option<String>,
    /// Only items of one type: `"dm"`, `"comment"`, or `"mention"`.
    pub r#type: Option<String>,
    /// `"oldest"` (the default: the item that has waited longest first) or
    /// `"newest"` (the most recent).
    pub order: Option<String>,
    /// Also serve items that were marked read but never answered. By default
    /// only unread items are served, so marking a conversation read is the
    /// durable way to skip it.
    pub include_read: Option<bool>,
    /// Conversation ids to leave out of this call (a session-local skip; up
    /// to 100). Sent comma-separated.
    pub exclude: Option<Vec<String>>,
}

// ─── Webhooks ────────────────────────────────────────────────────────────────

/// Body for `POST /webhooks`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateWebhookParams {
    /// HTTPS endpoint that will receive event deliveries.
    pub url: String,
    /// Events to subscribe to: `post.scheduled`, `post.published`,
    /// `post.failed`, `post.approved`, `post.rejected`, `ad.status_changed`
    /// (its `data` is the campaign, not a post), `ad.created`, `ad.updated`,
    /// `ad.review`, `ad.delivery_issue`, `campaign.created`,
    /// `campaign.updated`, `campaign.budget`, `lead.received`,
    /// `ads.sync_failed`, `ads.sync_recovered`,
    /// `ads.initial_sync_completed`, `ad_rule.triggered`. Any ads event needs
    /// the `ads:read` scope on the key and `access_ads` for its person, else
    /// `403 insufficient_scope` / `insufficient_permission`.
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
