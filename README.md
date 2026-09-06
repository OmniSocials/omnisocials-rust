# OmniSocials Rust SDK

The official Rust client for the [OmniSocials API](https://docs.omnisocials.com). Schedule and publish posts to Instagram, Facebook, LinkedIn, YouTube, TikTok, X, Pinterest, Bluesky, Threads, Mastodon, and Google Business from one API.

- Async-first, built on `reqwest` with rustls (no OpenSSL needed)
- Typed request params with serde, responses as `serde_json::Value`
- Automatic retries with exponential backoff, configurable timeouts
- A rich `Error` enum and a webhook signature verification helper

## Installation

```bash
cargo add omnisocials
```

You also need an async runtime such as [tokio](https://tokio.rs):

```bash
cargo add tokio --features rt-multi-thread,macros
```

## Quickstart

```rust
use omnisocials::{Client, CreatePostParams};

#[tokio::main]
async fn main() -> Result<(), omnisocials::Error> {
    let client = Client::from_env()?; // reads OMNISOCIALS_API_KEY
    let post = client.posts().create(CreatePostParams {
        content: "Hello from the SDK".into(),
        channels: Some(vec!["instagram".into(), "linkedin".into()]),
        scheduled_at: Some("2026-08-01T09:00:00Z".into()),
        ..Default::default()
    }).await?;
    println!("created {}", post["data"]["id"]);
    Ok(())
}
```

## Authentication

Create an API key in the OmniSocials app under **Settings -> API Keys**. Keys look like `omsk_live_...` (or `omsk_test_...`).

The client reads `OMNISOCIALS_API_KEY` from the environment, or you can pass one explicitly:

```rust
let client = omnisocials::Client::new("omsk_live_...")?;
```

Constructing a client without a key returns `Error::Auth` right away, with code `missing_api_key`.

## Configuration

```rust
use std::time::Duration;

let client = omnisocials::Client::builder()
    .api_key("omsk_live_...")                       // wins over the env var
    .base_url("https://api.omnisocials.com/v1")     // default
    .timeout(Duration::from_secs(30))               // per-request timeout (default 30s)
    .max_retries(2)                                 // retries on 429 / 5xx / network errors (default 2)
    .build()?;
```

Retries use exponential backoff (0.5s, 1s, 2s, ...) with jitter and honor the `Retry-After` header. Other 4xx responses are never retried.

## Rate limits

The API allows **100 requests per minute** per API key. When you exceed it, the SDK retries automatically (respecting `Retry-After`); if retries are exhausted it returns `Error::RateLimit` whose `retry_after` field holds the seconds to wait.

## Return values

Methods return the parsed response body as-is, as a `serde_json::Value`: single items come back as `{"data": {...}}`, lists as `{"data": [...], "pagination": {...}}`, and some responses carry extra sibling keys (media uploads include `compatibility`, PDF uploads include `slides` and `media_ids`, post creates targeting X with a URL in the text include `warnings`). Endpoints that respond `204 No Content` (deletes) resolve to `Value::Null`. Index into values with `post["data"]["id"]`, or deserialize into your own structs with `serde_json::from_value`.

## Posts

### Schedule a post

```rust
use omnisocials::CreatePostParams;

let post = client.posts().create(CreatePostParams {
    content: "New drop this Friday".into(),
    channels: Some(vec!["instagram".into(), "facebook".into(), "linkedin".into()]),
    scheduled_at: Some("2026-08-01T09:00:00Z".into()),
    media_urls: Some(vec!["https://example.com/teaser.jpg"].into()),
    ..Default::default()
}).await?;
println!("{} {}", post["data"]["id"], post["data"]["status"]);
```

Omit `scheduled_at` to create a draft. Use `Content::PerPlatform` for per-platform captions:

```rust
use std::collections::HashMap;
use omnisocials::{Content, CreatePostParams};

client.posts().create(CreatePostParams {
    content: Content::PerPlatform(HashMap::from([
        ("default".into(), "New drop this Friday".into()),
        ("x".into(), "New drop this Friday. RT to spread the word".into()),
    ])),
    channels: Some(vec!["instagram".into(), "x".into()]),
    scheduled_at: Some("2026-08-01T09:00:00Z".into()),
    ..Default::default()
}).await?;
```

### Publish immediately

```rust
client.posts().create_and_publish(CreatePostParams {
    content: "Going live right now".into(),
    channels: Some(vec!["x".into(), "bluesky".into()]),
    ..Default::default()
}).await?;
```

### Per-media alt text

Every `media_urls` / `media_ids` entry accepts either a plain string or a `MediaEntry` with an `alt` accessibility description (max 1500 chars): `MediaEntry::Url` for `media_urls`, `MediaEntry::Id` for `media_ids`. Alt text is delivered to Mastodon (media description), Bluesky (embed alt), X (photos and GIFs), Pinterest (pin alt text), Instagram (images), and LinkedIn (images). Plain and alt-carrying entries can be mixed via `MediaEntry::Plain`, and the same shape works in per-platform maps and `thread_parts` media.

```rust
use omnisocials::{CreatePostParams, MediaEntry};

client.posts().create(CreatePostParams {
    content: "Sunrise over the harbor".into(),
    channels: Some(vec!["mastodon".into(), "bluesky".into()]),
    scheduled_at: Some("2026-08-01T09:00:00Z".into()),
    media_urls: Some(vec![MediaEntry::Url {
        url: "https://example.com/harbor.jpg".into(),
        alt: Some("A small sailboat crossing a calm harbor at sunrise, sky in deep orange".into()),
    }].into()),
    ..Default::default()
}).await?;
```

### Post with platform-specific options

Platform option blocks are free-form JSON; build them with `serde_json::json!`:

```rust
use serde_json::json;

client.posts().create(CreatePostParams {
    content: "Behind the scenes of our summer shoot".into(),
    channels: Some(vec!["instagram".into(), "youtube".into(), "x".into()]),
    scheduled_at: Some("2026-08-01T09:00:00Z".into()),
    media_urls: Some(vec!["https://example.com/bts.mp4"].into()),
    instagram: Some(json!({ "share_to_feed": true })),
    youtube: Some(json!({ "title": "Summer shoot BTS", "privacy": "public" })),
    x: Some(json!({ "reply_settings": "following", "made_with_ai": false })),
    ..Default::default()
}).await?;
```

### Chained threads (X, Bluesky, Mastodon, Threads)

Provide 2 to 25 `thread_parts` to publish a chained thread instead of a single tweet. Each part is capped at 280 characters (25,000 for X Premium/Premium+ accounts) and can carry its own media (`media_ids` / `media_urls`, max 4 per part). The same `thread_parts` shape works for `bluesky` (300 chars per part), `mastodon` (500 chars per part) and `threads` (Meta Threads: 2 to 25 parts, 500 characters per part, up to 10 media per part; parts after the first publish as replies to the previous part, and the Threads caption is taken from part 1).

```rust
client.posts().create(CreatePostParams {
    content: "How we grew to 10k followers in 90 days".into(),
    channels: Some(vec!["x".into()]),
    scheduled_at: Some("2026-08-01T09:00:00Z".into()),
    x: Some(json!({
        "thread_parts": [
            { "text": "How we grew to 10k followers in 90 days. A thread:" },
            { "text": "1. We posted every single day, even when it felt pointless." },
            { "text": "2. We replied to every comment within an hour." },
            { "text": "3. Full breakdown on our blog. Link in bio." }
        ]
    })),
    ..Default::default()
}).await?;
```

```rust
// Meta Threads chain with a carousel on the first part
client.posts().create(CreatePostParams {
    content: "Behind the scenes of our summer shoot".into(),
    channels: Some(vec!["threads".into()]),
    threads: Some(json!({
        "thread_parts": [
            { "text": "Behind the scenes of our summer shoot. A few highlights:", "media_urls": ["https://example.com/shoot-1.jpg", "https://example.com/shoot-2.jpg"] },
            { "text": "Day one: scouting locations at sunrise." },
            { "text": "Day two: the full crew, 14 hours, zero regrets." }
        ]
    })),
    ..Default::default()
}).await?;
```

On update, pass `json!({ "thread_parts": null })` to clear thread mode (revert to a single post); omit the field to leave the existing thread untouched. The same applies to `bluesky`, `mastodon` and `threads`.

The `threads` options also accept `location_id`: a Threads location id from `locations().search_with` with platform `"threads"` (see [Locations](#locations-instagram-and-threads-place-tagging)). On a multi-part thread the tag is applied to part 1, and on update `json!({ "location_id": null })` clears the tag. Threads location tagging is currently rolling out; until Meta approves the permissions it is disabled on production and calls return a clear error.

### X link posts use credits

X bills API posts whose text contains a URL at a premium, and OmniSocials passes that fee through as prepaid credits (20 credits per URL-containing tweet; threads billed per part with a link). When a create targets X and the text contains a URL, the response carries a top-level `warnings` array (a sibling of `data`):

```rust
let res = client.posts().create(CreatePostParams {
    content: "Read the full story: https://example.com/post".into(),
    channels: Some(vec!["x".into()]),
    ..Default::default()
}).await?;
if let Some(warnings) = res["warnings"].as_array() {
    for warning in warnings {
        if warning["code"] == "x_url_post_credits" {
            println!("{} credits (balance: {})", warning["credits_required"], warning["credits_balance"]);
        }
    }
}
```

From `enforce_from` (2026-08-14) the balance is checked at publish time, but credits are only deducted after the post successfully publishes (a failed publish is never charged). If the balance can't cover it, only the X target fails (other platforms publish normally); top up in the dashboard under Settings -> Organisation -> Billing -> Credits, then call `posts().retry`. Posts without links, analytics, and media on X stay free. There is no API endpoint for credits — they are managed in the dashboard.

Separately, from 2026-08-14, `create`, `update`, and `publish` also gate up front: if reserving this post's cost would push the company's total reserved credits past its balance, the call is refused before it's ever accepted, with `Error::Api` (status 402) and code `x_credits_insufficient`. `error.details` on that response carries `credits_required`, `credits_balance`, and `credits_reserved`. Drafts (no `scheduled_at`) are never gated, and neither is a post scheduled to publish before 2026-08-14.

```rust
use omnisocials::Error;

match client.posts().create(CreatePostParams {
    content: "Read the full story: https://example.com/post".into(),
    channels: Some(vec!["x".into()]),
    scheduled_at: Some("2026-08-20T09:00:00Z".into()),
    ..Default::default()
}).await {
    Ok(post) => println!("scheduled {}", post["data"]["id"]),
    Err(err) if err.code() == Some("x_credits_insufficient") => {
        eprintln!("not enough reserved credits, top up in the dashboard");
    }
    Err(err) => eprintln!("create failed: {err}"),
}
```

### List, get, update, publish, retry, delete

```rust
use omnisocials::{ListPostsParams, UpdatePostParams};

let posts = client.posts().list(ListPostsParams {
    status: Some("scheduled".into()),
    limit: Some(50),
    ..Default::default()
}).await?;
let id = posts["data"][0]["id"].as_str().unwrap();

let one = client.posts().get(id).await?;
client.posts().update(id, UpdatePostParams {
    scheduled_at: Some("2026-08-02T10:00:00Z".into()),
    ..Default::default()
}).await?;
client.posts().publish(id).await?;      // publish a draft/scheduled post now
client.posts().retry(id).await?;        // retry only the failed platforms of a failed/warning post
client.posts().delete(id).await?;       // resolves to Value::Null (204)
```

`retry` re-publishes only the platforms that failed, on the same post; platforms that already succeeded are never posted again. It is asynchronous: a 200 means the retry is queued, so poll `get` for the outcome. Max 3 retries per platform.

### Approve or reject a post

```rust
client.posts().approve(id).await?;                                    // approve the current approval-workflow step
client.posts().reject(id, Some("Wrong CTA link, please fix.")).await?; // reject and stop the workflow (pass None for no comment)
```

Only works on a post with `approval_status: "pending"` (`status: "in_approval"`). Both act on behalf of the user who owns the API key, who must be a listed approver for the workflow's CURRENT step — steps approve in order, so being an approver on a later step is not enough yet (returns `Error::Api` with code `forbidden`). Approving the last step finalizes the post (`scheduled` or `posting`); rejecting stops the whole workflow immediately, not just the current step.

### Recent platform posts

Fetch recent posts live from the connected platform APIs, including content published outside OmniSocials. Useful for brand-new workspaces where `list` is empty. Requires the `analytics:read` scope. Each record includes `duration_seconds` (integer, nullable): the video length in whole seconds where the platform reports it — currently TikTok and YouTube; `null` for images and for platforms that don't expose it.

```rust
use omnisocials::RecentPlatformPostsParams;

let recent = client.posts().recent_platform(RecentPlatformPostsParams {
    limit: Some(10),
    platforms: Some(vec!["instagram".into(), "x".into()]),
}).await?;
```

## Media

### Upload from a URL (recommended, up to 1GB)

```rust
use omnisocials::UploadMediaFromUrlParams;

let upload = client.media().upload_from_url(UploadMediaFromUrlParams {
    url: "https://example.com/launch-video.mp4".into(),
    name: Some("launch-video-v2".into()),
    folder: Some("Campaigns".into()),
    ..Default::default()
}).await?;
println!("{} {:?}", upload["data"]["id"], upload["compatibility"]);
```

Videos over 100MB are processed in the background and come back with status `"processing"`. Every upload response includes a `compatibility` block listing connected platforms that would reject the file.

### Upload local bytes (multipart)

```rust
use omnisocials::UploadMediaParams;

let bytes = std::fs::read("./photos/product.jpg")?;
let uploaded = client.media().upload(UploadMediaParams {
    file: bytes,
    filename: "product.jpg".into(),
    name: Some("product-hero".into()),
    ..Default::default()
}).await?;
```

Direct multipart uploads are capped at 100MB by the CDN; use `upload_from_url` or the presigned flow below for bigger files.

### Upload from base64

```rust
use omnisocials::UploadMediaFromBase64Params;

client.media().upload_from_base64(UploadMediaFromBase64Params {
    data: base64_string, // no data URI prefix
    mime_type: "image/png".into(),
    filename: Some("chart.png".into()),
    ..Default::default()
}).await?;
```

### PDF carousels

Uploading a PDF rasterizes it into one image slide per page (max 20). The response carries `slides` and `media_ids` alongside `data` (the first slide). Pass ALL of `media_ids`, in order, to `posts().create` to post the deck as a carousel (a native swipeable document on LinkedIn, an image carousel elsewhere).

```rust
let pdf = client.media().upload_from_url(UploadMediaFromUrlParams {
    url: "https://example.com/deck.pdf".into(),
    ..Default::default()
}).await?;
let media_ids: Vec<String> = serde_json::from_value(pdf["media_ids"].clone()).unwrap();

client.posts().create(CreatePostParams {
    content: "Our Q3 strategy deck".into(),
    channels: Some(vec!["linkedin".into()]),
    media_ids: Some(media_ids.into()),
    scheduled_at: Some("2026-08-01T09:00:00Z".into()),
    ..Default::default()
}).await?;
```

### Presigned uploads for large files (up to 1GB)

`create_upload_url` mints a one-time upload URL. POST the file to it as multipart form data (field name `file`) within `expires_in_seconds` (600s); the second request needs no auth headers because the single-use token is in the URL. The response of that second request is the created media item (or `media_ids` for a PDF).

```rust
let minted = client.media().create_upload_url().await?;
let upload_url = minted["upload_url"].as_str().unwrap();

// Second leg with any HTTP client, e.g. reqwest:
let form = reqwest::multipart::Form::new().part(
    "file",
    reqwest::multipart::Part::bytes(std::fs::read("./big-video.mp4")?)
        .file_name("big-video.mp4"),
);
let uploaded: serde_json::Value = reqwest::Client::new()
    .post(upload_url)
    .multipart(form)
    .send()
    .await?
    .json()
    .await?;
println!("{}", uploaded["data"]["id"]);
```

### Preflight compatibility check

Check a file against the workspace's connected platforms before uploading. Provide one of `url`, `media_id`, or `size_bytes` + `mime`.

```rust
use omnisocials::CheckMediaParams;

client.media().check(CheckMediaParams {
    url: Some("https://example.com/huge.mov".into()),
    ..Default::default()
}).await?;

client.media().check(CheckMediaParams {
    size_bytes: Some(300_000_000),
    mime: Some("video/quicktime".into()),
    ..Default::default()
}).await?;
```

### List, get, rename, move, delete

```rust
use omnisocials::{ListMediaParams, UpdateMediaParams};

let items = client.media().list(ListMediaParams {
    search: Some("hero".into()),
    limit: Some(20),
    ..Default::default()
}).await?;
let id = items["data"][0]["id"].as_str().unwrap();

client.media().update(id, UpdateMediaParams {
    name: Some("hero-v2".into()),
    folder_id: Some(Some("12".into())), // Some(None) sends null: move to root
}).await?;
client.media().get(id).await?;
client.media().delete(id).await?; // 409 media_in_use if attached to a scheduled post
```

## Folders

```rust
use omnisocials::{CreateFolderParams, UpdateFolderParams};

let folders = client.folders().list().await?; // flat; build the tree via parent_id
let folder = client.folders().create(CreateFolderParams {
    name: "Campaigns".into(),
    parent_id: None,
}).await?;
let id = folder["data"]["id"].as_str().unwrap();

client.folders().update(id, UpdateFolderParams {
    name: Some("Campaigns 2026".into()),
    parent_id: None, // Some(None) sends null: move to the top level
}).await?;
client.folders().delete(id).await?; // files move to root, subfolders move up
```

## Hashtag Sets

Save reusable hashtag groups and apply them to posts at create time. Uses the `posts:read` / `posts:write` scopes.

```rust
use omnisocials::{CreateHashtagSetParams, UpdateHashtagSetParams};

let set = client.hashtag_sets().create(CreateHashtagSetParams {
    name: "Launch".into(),
    hashtags: vec!["saas", "buildinpublic", "startup"].into(), // or one string: "#saas #buildinpublic #startup".into()
}).await?;
let id = set["data"]["id"].as_str().unwrap();
println!("{}", set["data"]["preview"]); // "#saas #buildinpublic #startup"

client.hashtag_sets().list().await?;
client.hashtag_sets().get(id).await?;
client.hashtag_sets().update(id, UpdateHashtagSetParams {
    hashtags: Some(vec!["saas", "founder"].into()), // replaces the full list
    ..Default::default()
}).await?;
client.hashtag_sets().delete(id).await?; // resolves to Value::Null (204)
```

Apply a set when creating a post with `hashtag_set` (the set name, case-insensitive) or `hashtag_set_id`. The set is applied once at create time and tags already in the caption are skipped. `hashtag_placement` is `"caption_append"` (default) or `"first_comment"`, and `hashtag_platforms` restricts the hashtags to a subset of the post's channels. Instagram's 30-hashtag cap returns error code `hashtag_limit_exceeded`.

```rust
client.posts().create(CreatePostParams {
    content: "Launch day!".into(),
    channels: Some(vec!["instagram".into(), "x".into()]),
    scheduled_at: Some("2026-08-01T09:00:00Z".into()),
    hashtag_set: Some("Launch".into()),
    hashtag_placement: Some("first_comment".into()),
    hashtag_platforms: Some(vec!["instagram".into()]),
    ..Default::default()
}).await?;
```

## Accounts

```rust
let accounts = client.accounts().list().await?;
for account in accounts["data"].as_array().unwrap() {
    println!("{} {} {}", account["platform"], account["username"], account["status"]);
    if account["needs_reconnect"] == true {
        eprintln!("{} needs a reconnect: {}", account["platform"], account["reauth_reason"]);
    }
}
let first_id = accounts["data"][0]["id"].as_str().unwrap();
let one = client.accounts().get(first_id).await?;
```

## Analytics

```rust
use omnisocials::{AccountAnalyticsParams, AnalyticsOverviewParams};

// One post's latest per-platform metrics
let stats = client.analytics().post("post_id").await?;
println!("{:?}", stats["data"]["platforms"]["instagram"]["metrics"]);

// Batch: up to 100 posts in one call
let batch = client.analytics().posts(&["id1", "id2", "id3"]).await?;

// Workspace-wide overview
let overview = client.analytics().overview(AnalyticsOverviewParams {
    period: Some("30d".into()),
    ..Default::default()
}).await?;
println!(
    "{} impressions, {} engagements",
    overview["data"]["total_impressions"], overview["data"]["total_engagements"]
);

// Account-level stats (followers etc)
let account_stats = client.analytics().accounts(AccountAnalyticsParams {
    platform: Some("instagram".into()),
    ..Default::default()
}).await?;
```

### Best times to post

```rust
use omnisocials::BestTimesParams;

let best = client.analytics().best_times(BestTimesParams {
    platform: "instagram".into(),
    timezone: Some("Europe/Amsterdam".into()),
}).await?;
```

## Inbox

Read and reply to social conversations, DMs, comments, and mentions, across Instagram, Facebook, LinkedIn, TikTok (video comments only), YouTube (video comments only), X, and Threads. TikTok and YouTube replies are comments only; TikTok replies are capped at 150 characters. Threads conversations are `type` `"comment"` (replies people leave on your Threads posts; conversation ids look like `threads_comment_<rootPostId>`) and `"mention"` (`threads_mention_<postId>`); there are no Threads DMs, and a reply publishes as a native Threads reply. The Threads inbox is currently rolling out; until Meta approves the permissions it is disabled on production and calls return a clear error, and it needs a Threads connection with the reply permission (a connection without it fails with `Error::Auth`, status 401, code `reauth_required`; reconnect Threads to fix it).

```rust
use omnisocials::{ListConversationsParams, ReplyParams};

let conversations = client.inbox().list_conversations(ListConversationsParams {
    platform: Some("instagram".into()),
    unread: Some(true),
    ..Default::default()
}).await?;
let conversation_id = conversations["data"][0]["id"].as_str().unwrap();

let messages = client.inbox().get_messages(conversation_id, Default::default()).await?;
client.inbox().mark_read(conversation_id).await?;
client.inbox().reply(conversation_id, ReplyParams {
    text: "Thanks for reaching out! We'll get back to you shortly.".into(),
    ..Default::default()
}).await?;

// Hide (or unhide) a comment someone left on one of your posts, as the post
// owner: Facebook, Instagram, TikTok, YouTube and Threads comments (Threads:
// incoming top-level replies only). The message keeps its place in the
// conversation and comes back with `hidden` flipped.
let message_id = messages["data"][0]["id"].as_str().unwrap();
client.inbox().hide(message_id, true).await?;  // hide
client.inbox().hide(message_id, false).await?; // unhide

// Delete a comment outright (Facebook, Instagram, TikTok; YouTube: hide
// instead). Replies under it go with it; their ids come back as
// `removed_reply_ids`.
client.inbox().delete_message(message_id).await?;
```

### Work queue: what needs an answer

`inbox().next()` hands out the next conversation that still needs a reply (the customer's latest DM with no reply after it, or an unreplied comment/mention that is not hidden), with the whole thread and the post it belongs to (`post.url`, `post.media_type`), so a reply can be drafted from one call. Replies typed in the native apps count as answers. Only unread items are served by default, so `mark_read` is the durable way to skip one; `exclude` skips conversation ids for the current session only. Set `include_next: Some(true)` on `reply` to get the following item in the same response. `ListConversationsParams { unanswered: Some(true), .. }` gives the same set as a plain list.

```rust
use omnisocials::NextUnansweredParams;

let mut next = client.inbox().next(NextUnansweredParams {
    platform: Some("instagram".into()),
    ..Default::default()
}).await?;
while !next["data"].is_null() {
    let message = &next["data"]["message"];
    println!("{} left. {}: {}", next["remaining"], message["sender"]["username"], message["text"]);

    let reply = client.inbox().reply(message["conversation_id"].as_str().unwrap(), ReplyParams {
        text: "Thanks! DM sent.".into(),
        include_next: Some(true),
        ..Default::default()
    }).await?;
    next = serde_json::json!({ "data": reply["next"], "remaining": reply["remaining"] });
}
```

`list_conversations` and `get_messages` are cursor-paginated: read `pagination.next_cursor` from the response and pass it back as `cursor` to fetch the next page, and stop once `pagination.has_more` is `false`.

### X DM replies use credits

Replying to an X DM costs 2 prepaid credits per send, debited before the send and automatically refunded if the send fails. If the balance can't cover it, `reply` fails with `Error::Api` (status 402) and code `insufficient_credits`. If the workspace's X inbox was auto-suspended for hitting a zero balance, it fails with code `x_inbox_suspended` instead; top up and re-enable the inbox in the dashboard to resume (DMs that arrived while suspended are not recovered).

```rust
use omnisocials::Error;

match client.inbox().reply(conversation_id, ReplyParams {
    text: "On our way!".into(),
    ..Default::default()
}).await {
    Ok(reply) => println!("sent {}", reply["data"]["id"]),
    Err(err) if err.code() == Some("insufficient_credits") => {
        eprintln!("not enough credits, top up in the dashboard");
    }
    Err(err) if err.code() == Some("x_inbox_suspended") => {
        eprintln!("X inbox suspended, top up and re-enable it");
    }
    Err(err) => eprintln!("reply failed: {err}"),
}
```

## Locations (Instagram and Threads place tagging)

```rust
let results = client.locations().search("Griffith Observatory").await?;
let place_id = results["data"][0]["id"].as_str().unwrap();

let check = client.locations().validate(place_id).await?;
if check["valid"] == true {
    client.posts().create(CreatePostParams {
        content: "Golden hour at the observatory".into(),
        channels: Some(vec!["instagram".into()]),
        media_urls: Some(vec!["https://example.com/observatory.jpg"].into()),
        location_id: Some(place_id.into()),
        scheduled_at: Some("2026-08-01T18:30:00Z".into()),
        ..Default::default()
    }).await?;
}
```

Use `search_with` with platform `"threads"` to search Meta's Threads location catalog instead of Facebook Places (the two sources use different ids). With platform `"threads"` you can also search around a point with `latitude` + `longitude` instead of `q` (pass either `q` or the coordinate pair). The Threads response is `{"locations": [...]}` (each entry: `id`, plus nullable `name`, `address`, `city`, `country`, `latitude`, `longitude`) or `{"error": {"code", "message"}}` with code `not_available`, `threads_not_connected`, `threads_reauth_required` (reconnect Threads), or `platform_error`. Use a result's `id` as `threads.location_id` in the post options, not as the top-level `location_id`. Threads location tagging is currently rolling out; until Meta approves the permissions it is disabled on production and calls return a clear error.

```rust
use omnisocials::SearchLocationsParams;

let places = client.locations().search_with(SearchLocationsParams {
    q: Some("Griffith Observatory".into()),
    platform: Some("threads".into()),
    ..Default::default()
}).await?;
// Or search around a point instead of by text:
let nearby = client.locations().search_with(SearchLocationsParams {
    platform: Some("threads".into()),
    latitude: Some(34.1184),
    longitude: Some(-118.3004),
    ..Default::default()
}).await?;
let threads_location_id = places["locations"][0]["id"].as_str().unwrap();

client.posts().create(CreatePostParams {
    content: "Golden hour at the observatory".into(),
    channels: Some(vec!["threads".into()]),
    media_urls: Some(vec!["https://example.com/observatory.jpg"].into()),
    threads: Some(json!({ "location_id": threads_location_id })),
    scheduled_at: Some("2026-08-01T18:30:00Z".into()),
    ..Default::default()
}).await?;
```

## Webhooks

### Manage endpoints

```rust
use omnisocials::{CreateWebhookParams, UpdateWebhookParams};

let webhook = client.webhooks().create(CreateWebhookParams {
    url: "https://example.com/omnisocials/webhook".into(),
    events: vec!["post.published".into(), "post.failed".into()],
}).await?;
println!("{}", webhook["data"]["secret"]); // save it, it is only shown once
let id = webhook["data"]["id"].as_str().unwrap();

client.webhooks().list().await?;
client.webhooks().get(id).await?;
client.webhooks().update(id, UpdateWebhookParams {
    is_active: Some(false),
    ..Default::default()
}).await?;
let rotated = client.webhooks().rotate_secret(id).await?;
println!("{}", rotated["data"]["secret"]); // the old secret stops working
client.webhooks().delete(id).await?;
```

### Verify deliveries (axum example)

Every delivery is signed with your webhook secret. The `X-OmniSocials-Signature` header has the form `t=<unix>,v1=<hex>` where the hex value is an HMAC-SHA256 of `"{timestamp}.{rawBody}"`. Always verify against the RAW request body. The example uses [axum](https://crates.io/crates/axum) (not a dependency of this crate):

```rust
use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Router;

async fn omnisocials_webhook(headers: HeaderMap, body: Bytes) -> StatusCode {
    let signature = headers
        .get("x-omnisocials-signature")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let secret = std::env::var("OMNISOCIALS_WEBHOOK_SECRET").expect("secret not set");

    // `body` is the raw request bytes: exactly what the signature covers.
    match omnisocials::webhooks::verify_signature(&body, signature, &secret, 300) {
        Ok(event) => {
            match event["type"].as_str() {
                Some("post.published") => {
                    println!("Published: {} {:?}", event["data"]["post_id"], event["data"]["targets"]);
                }
                Some("post.failed") => {
                    eprintln!("Failed: {}", event["data"]["post_id"]);
                }
                _ => {}
            }
            StatusCode::OK
        }
        Err(_) => StatusCode::BAD_REQUEST,
    }
}

let app: Router = Router::new().route("/omnisocials/webhook", post(omnisocials_webhook));
```

`verify_signature` uses a constant-time comparison, rejects timestamps older than the tolerance (300 seconds here; replay protection), returns `Error::WebhookVerification` on any failure, and returns the parsed event on success.

## Health

```rust
let health = client.health().await?; // {"status": "ok", "version": "1.0.0", "timestamp": "..."}
```

## Error handling

Every method returns `Result<serde_json::Value, omnisocials::Error>`. Non-2xx API responses map to the most specific variant, each carrying the `status`, the machine-readable `code` from the API body, the `message`, and the parsed `body`:

| Variant | Status | Typical API codes |
|---|---|---|
| `Error::Validation` | 400 / 422 | `validation_error`, `platform_not_connected`, `invalid_file_type` |
| `Error::Auth` | 401 | `unauthorized`, `invalid_api_key` |
| `Error::PermissionDenied` | 403 | `forbidden`, `insufficient_scope` |
| `Error::NotFound` | 404 | `not_found` |
| `Error::RateLimit` | 429 | `rate_limit_exceeded` (exposes `retry_after` seconds) |
| `Error::Server` | >= 500 | `internal_error` |
| `Error::Api` | other non-2xx | e.g. 409 `media_in_use` |
| `Error::Connection` | n/a | network failure or timeout |
| `Error::WebhookVerification` | n/a | invalid webhook signature |

```rust
use omnisocials::{CreatePostParams, Error};

match client.posts().create(CreatePostParams {
    content: "Hi".into(),
    channels: Some(vec!["instagram".into()]),
    ..Default::default()
}).await {
    Ok(post) => println!("created {}", post["data"]["id"]),
    Err(Error::RateLimit { retry_after, .. }) => {
        eprintln!("rate limited, retry in {:?}s", retry_after);
    }
    Err(Error::Validation { code, message, .. }) => {
        eprintln!("bad request ({:?}): {}", code, message);
    }
    Err(Error::Connection(err)) => {
        eprintln!("network problem: {err}");
    }
    Err(err) => {
        eprintln!("API error {:?} ({:?}): {}", err.status(), err.code(), err);
    }
}
```

The `err.status()`, `err.code()`, and `err.retry_after()` accessors work across all API variants.

## API scopes

Each API key carries scopes: `posts:read`, `posts:write`, `media:write`, `accounts:read`, `analytics:read`, `webhooks:manage`. A call with a missing scope returns `Error::PermissionDenied` with code `insufficient_scope`.

## Documentation

Full API reference and guides: [https://docs.omnisocials.com](https://docs.omnisocials.com)

## License

MIT
