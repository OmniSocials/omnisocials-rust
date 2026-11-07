# OmniSocials Rust SDK

The official Rust SDK for OmniSocials: post, schedule, reply and report on 13 social platforms (Instagram, Facebook, LinkedIn, X, TikTok, YouTube, Threads, Pinterest, Bluesky, Mastodon, Google Business, Telegram and Discord) from one API. Guide: [omnisocials.com/sdk/rust](https://omnisocials.com/sdk/rust)

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

Create an API key in the OmniSocials app under **Settings -> API Keys**. Keys start with `omsk_live_`. There are no test keys and no test mode: every key works on your real connected accounts. To try things safely, leave out `scheduled_at` so the post stays a draft (a draft never publishes on its own), or use a separate workspace with test accounts.

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

### Planned drafts

Set `draft: Some(true)` with `scheduled_at` to store the post as a draft that keeps its time. It shows on the dashboard calendar at that slot with a dashed outline and the scheduler never publishes it: a person schedules it from the composer, or you do with `update` and `draft: Some(false)`. Use it when an agent proposes a slot that a human should confirm. A draft is not allowed with `publish_now` or `approval_workflow_id` (`400 validation_error`). On `update`, `draft: Some(false)` without `scheduled_at` is a `400 validation_error` when the draft has no time or its time is in the past (pass a new `scheduled_at` then), and `draft: Some(true)` on a scheduled post unschedules it while keeping the time.

```rust
use omnisocials::{CreatePostParams, UpdatePostParams};

let proposal = client.posts().create(CreatePostParams {
    content: "Proposed for Friday morning".into(),
    channels: Some(vec!["instagram".into(), "linkedin".into()]),
    scheduled_at: Some("2026-08-01T09:00:00Z".into()),
    draft: Some(true),
    ..Default::default()
}).await?;
println!("{} {}", proposal["data"]["status"], proposal["data"]["schedule_at"]); // draft, the time is kept

// The human agrees: schedule it at the time it already carries
let id = proposal["data"]["id"].as_str().unwrap();
client.posts().update(id, UpdatePostParams { draft: Some(false), ..Default::default() }).await?;
```

### Comment-to-DM automations

Answer a keyword in an Instagram comment with a DM, once per person, with an optional public reply under the comment. Set `comment_dm_automation` on `create` (Instagram must be in `channels`) and the automation is bound to that post; it starts listening once the post is live. Needs the opt-in `automations:write` scope on the key. `client.automations()` manages the automations themselves (`list`, `get`, `create`, `update`, `delete`) and reads their activity (`list_events`, `automations:read`). On `update`, `comment_dm_automation: Some(json!({...}))` replaces the post's automation and `Some(json!(null))` removes it. Instagram rules: one private reply per comment, within 7 days; OmniSocials caps sends at 180 DMs an hour per account.

```rust
use omnisocials::{CreatePostParams, ListAutomationEventsParams};
use serde_json::json;

let post = client.posts().create(CreatePostParams {
    content: "Comment GUIDE and I will send you the link".into(),
    channels: Some(vec!["instagram".into()]),
    scheduled_at: Some("2026-08-01T09:00:00Z".into()),
    media_urls: Some(vec!["https://example.com/guide-cover.jpg"].into()),
    comment_dm_automation: Some(json!({
        "trigger": { "keywords": ["guide"], "match": "word" },
        "actions": {
            "dm": { "text": "Here is the guide: https://example.com/guide" },
            "public_reply": { "enabled": true, "variations": ["Sent you a DM!", "Check your inbox"] }
        },
        "limits": { "max_sends": 500 }
    })),
    ..Default::default()
}).await?;
let automation_id = post["data"]["comment_dm_automation"]["id"].as_str().unwrap();

// Later: who was answered, and why someone was skipped
let events = client.automations().list_events(automation_id, ListAutomationEventsParams {
    outcome: Some("sent".into()),
    ..Default::default()
}).await?;
println!("{}", events["counts"]); // {"sent":12,"skipped":3}
for event in events["data"].as_array().unwrap() {
    println!("{} {}", event["person"]["username"], event["outcome"]);
}
```

The events response is `data`, `counts` and `pagination`, not the plain list: `counts` is the number of events per outcome (`sent`, `skipped`, `failed`, `pending`, `queued`, `opted_out`, `waiting`). An automation can also listen on a post published outside OmniSocials (`client.automations().create(CreateAutomationParams { instagram_media_id: Some(...), actions: json!({...}), ..Default::default() })`), on the next post the workspace publishes (`trigger: Some(json!({"scope": "next_post"}))`) or on every post (`"scope": "any"`), and on DMs and story replies (`"kinds": ["dm", "story_reply"]`).

Follow first (`"follow_gate"` in the actions, Instagram only) sends the link only to people who follow the account. On a comment the private reply is a first message with one button; a tap makes OmniSocials ask Instagram whether the person follows. A follower gets the DM text, anyone else gets the `not_following` message with its button, and every later tap checks again. A DM or a story reply is checked at once. When Instagram gives no answer the link goes out anyway. Texts hold 640 characters and buttons 20; leave them out for the defaults. A person who has not followed yet is an event with outcome `waiting` and `event["follow_gate"]` (`state`, `taps`, `followed`, `checked`, `after_ask`); `stats.followed` counts the people who followed after the ask. `"enabled": true` on a Facebook or X automation is a `400 follow_gate_instagram_only`.

```rust
use omnisocials::UpdateAutomationParams;

client.automations().update(automation_id, UpdateAutomationParams {
    actions: Some(json!({
        "dm": { "text": "Here is the guide: https://example.com/guide" },
        "follow_gate": {
            "enabled": true,
            "first_message": { "text": "Tap the button and I'll send you the link 👇", "button": "Send me the link" },
            "not_following": { "text": "Almost there! Follow @yourbrand first, then tap the button again.", "button": "I follow now" }
        }
    })),
    ..Default::default()
}).await?;
```

A Facebook Page works the same way, and the DM lands in Messenger: set `platform: Some("facebook".into())` on `CreateAutomationParams`, or `facebook_post_id` for a Page post published outside OmniSocials. On a post, `comment_dm_automation: Some(json!({"platforms": ["instagram", "facebook"], ...}))` makes one automation per channel with the same keywords and DM. Facebook has no story replies.

On X, set `platform: Some("x".into())` and `trigger: Some(json!({"kinds": ["dm"], "keywords": ["guide"]}))` on `CreateAutomationParams`: the X account answers a keyword in a DM, once per person, and every answer ends with "Reply STOP to stop automatic messages." An active X automation needs X DMs on for the workspace (`409 x_dms_off`; `status: Some("paused".into())` always saves). An answer over the send cap waits in a queue (`queued`) for up to an hour.

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

### Telegram channels and groups

`telegram` is a channel or group the workspace connected in OmniSocials; the OmniSocials bot posts to it. The `telegram` options: `silent`, `link_preview`, `pin`, `protect_content`, `spoiler` and `caption_above_media` (booleans), up to 3 `buttons` (`{ text, url }`, not on an album or a poll), a native `poll` (2 to 12 options of up to 100 characters, `open_period` 5 to 2628000 seconds, `multiple`; the post text is the question, max 300 characters, no media) and `first_comment` (a reply under the post). Text up to 4,096 characters; with media, text over 1,024 characters is sent as a reply under the media. Albums hold up to 10 photos and videos, files up to 50 MB, no Stories. Once sent, the post's `telegram.result` carries the message ids and the link. Telegram gives bots no view counts: analytics carry the reactions and the poll votes.

```rust
use serde_json::json;

client.posts().create(CreatePostParams {
    content: "The new collection is live. Tap the button to shop.".into(),
    channels: Some(vec!["telegram".into()]),
    scheduled_at: Some("2026-11-02T09:00:00Z".into()),
    media_urls: Some(vec!["https://example.com/collection.jpg"].into()),
    telegram: Some(json!({
        "silent": true,
        "pin": true,
        "buttons": [{ "text": "Shop now", "url": "https://example.com/shop" }],
        "first_comment": "Questions? Reply to this message."
    })),
    ..Default::default()
}).await?;

// A native poll: the text is the question.
client.posts().create(CreatePostParams {
    content: "Which day suits the live Q&A?".into(),
    channels: Some(vec!["telegram".into()]),
    telegram: Some(json!({ "poll": { "options": ["Tuesday", "Thursday", "Saturday"], "open_period": 86400 } })),
    ..Default::default()
}).await?;
```

### Discord channels

`discord` is one Discord channel the workspace connected in OmniSocials; the post goes out through the channel's incoming webhook and shows the webhook's name and picture. The `discord` options: `silent`, `link_preview` (default on), `allow_mentions` (default off, so @everyone, @here, role and user mentions show as text and ping nobody) and `spoiler` (booleans), `thread_name` (the post title in a forum or media channel, 1 to 100 characters), up to 5 `buttons` (`{ text, url }`, one row, not with a poll) and a native `poll` (2 to 10 options of up to 55 characters, `duration_hours` 1 to 768, default 24, `multiple`; the post text is the question, max 300 characters, no media). Text up to 2,000 characters. Up to 10 images, GIFs and videos per message; a video or GIF over Discord's 20 MB attachment limit is sent as a link in the text. No Stories. Once sent, the post's `discord.result` carries the message id and the link. Discord gives webhooks no view counts: analytics carry the reactions and the poll votes.

```rust
use serde_json::json;

client.posts().create(CreatePostParams {
    content: "Patch 2.4 is out. Read the notes below.".into(),
    channels: Some(vec!["discord".into()]),
    scheduled_at: Some("2026-11-02T09:00:00Z".into()),
    media_urls: Some(vec!["https://example.com/patch-notes.png"].into()),
    discord: Some(json!({
        "silent": true,
        "buttons": [{ "text": "Patch notes", "url": "https://example.com/patch-2-4" }]
    })),
    ..Default::default()
}).await?;

// A native poll: the text is the question.
client.posts().create(CreatePostParams {
    content: "Which mode do we test next?".into(),
    channels: Some(vec!["discord".into()]),
    discord: Some(json!({ "poll": { "options": ["Ranked", "Co-op", "Arena"], "duration_hours": 48 } })),
    ..Default::default()
}).await?;
```

On update, `UpdatePostParams { discord: Some(json!({ "poll": null })), .. }` merges into the stored options and removes the poll; `discord: Some(json!(null))` removes every Discord option.

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

The `threads` options also accept `topic_tag`: the topic of the post on Threads, for example `json!({ "topic_tag": "Build in public" })`. Unlike a #tag in the text, a topic can hold spaces. 1 to 50 characters, no period and no ampersand; a leading `#` is dropped. On a multi-part thread the topic is applied to part 1, and on update `json!({ "topic_tag": null })` removes it.

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

These calls return `Error::Api` (status 409) and change nothing when the post is not in a state for them:

- `post_publishing`: `update` or `delete` while the post is publishing (status `posting`) or a retry publishes one of its channels. Poll `get` until the status is `published`, `warning` or `failed`. An `update` with only `tags` still works.
- `review_required`: changes were requested on the post (`approval_status` `rejected`), so `publish`, or an `update` that would schedule it, is refused. It needs a new review: create it again with `approval_workflow_id`. Saving it as a draft works.
- `retry_in_progress`: `retry` while another retry of the post runs.
- `media_processing`: a media id in `create`, `create_and_publish` or `update` is still processing. Wait until `client.media().get(id)` says `ready`, then send the same request again. A file whose processing failed answers `400 invalid_media`: upload it again.

### Approve or reject a post

```rust
use omnisocials::{ApprovePostParams, Error, RejectPostParams};

// Read the step you review first, then name it on the decision.
let review = client.posts().get_approval(id).await?;
let step_order = review["data"]["current_step"].as_u64().map(|step| step as u32);

match client.posts().approve_with(id, ApprovePostParams { step_order }).await {
    Ok(_) => println!("approved"),
    Err(Error::Api { code: Some(code), body, .. }) if code == "step_changed" => {
        // Somebody decided since you read it: nothing was decided. Read the approval again.
        let now = body.map(|b| b["error"]["current_step_order"].clone());
        println!("the review is on step {now:?}");
    }
    Err(err) => return Err(err),
}

// Or reject and stop the workflow (comment optional):
// client.posts().reject_with(id, RejectPostParams {
//     comment: Some("Wrong CTA link, please fix.".into()),
//     step_order,
// }).await?;
```

Only works on a post with `approval_status: "pending"` (`status: "in_approval"`). Both act on behalf of the user who owns the API key, who must be a listed approver for the workflow's CURRENT step. Steps approve in order, so being an approver on a later step is not enough yet (returns `Error::Api` with code `forbidden`). Approving the last step finalizes the post (`scheduled` or `posting`); rejecting stops the whole workflow immediately, not just the current step.

`step_order` is optional: the `current_step` of `get_approval`. When the review is on another step now (somebody decided since you read it, or a retried call already went through), nothing is decided and the call returns `Error::Api` (status 409) with code `step_changed` and `error.current_step_order` in its `body`, the step the review is on now. Without it the call acts on whatever step is current, so a retry after a timeout could approve the next step too. `approve(id)` and `reject(id, comment)` still work and send no step.

### Read the approval review

```rust
let review = client.posts().get_approval(id).await?;
let data = &review["data"];
if data["status"] == "rejected" {
    println!("Rejected by {}: {}", data["rejection"]["by"]["name"], data["rejection"]["reason"]);
}
for step in data["steps"].as_array().into_iter().flatten() {
    println!("{} {} {}", step["order"], step["name"], step["status"]);
}
```

`get_approval` returns the review of a post that went through an approval workflow: `status` (`none`, `pending`, `approved`, `rejected`), the `workflow`, who requested it and when, `current_step` (the step the post waits on, `null` when the review ended), every step with its approvers and their decisions, the `rejection` (`by`, `reason`, `at`, `step`; `null` when nobody rejected) and the `comments` thread, oldest first. A post without an approval workflow returns `status: "none"` with empty `steps` and `comments`. Read-only; needs the `posts:read` scope.

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

Videos over 100MB are processed in the background and come back with status `"processing"`; when such a file cannot be queued, the call returns a 503 `processing_unavailable` error (send the same request again in a minute). Every upload response includes a `compatibility` block listing connected platforms that would reject the file.

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

Uploading a PDF rasterizes it into one image slide per page (max 20). The response carries `slides` and `media_ids` alongside `data` (the first slide). Pass ALL of `media_ids`, in order, to `posts().create` to post the deck as a carousel (on LinkedIn a native swipeable document made from the original PDF file, which is kept so text and links stay intact; an image carousel elsewhere). To keep the PDF as ONE library item instead of one item per page, pass `pdf_mode` = `"document"`: the response then has a single `data` item of type `document` (its page images in `pdf.pages`) and `media_ids` holds that one id, which expands into every page at post time.

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

## Post tags

Tags are your own labels for posts, such as a hook style or a format, to filter and compare posts in Analytics. They never go to a platform. Uses the `posts:read` / `posts:write` scopes (`analytics().tags()` needs `analytics:read`).

```rust
use omnisocials::{
    AnalyticsOverviewParams, ApplyTagsParams, CreateTagParams, ListPostsParams, TagAnalyticsParams,
    UpdatePostParams, UpdateTagParams,
};

let tag = client.tags().create(CreateTagParams {
    name: "Question hook".into(),
    color: Some("blue".into()), // optional
}).await?;
let id = tag["data"]["id"].as_i64().unwrap();

client.tags().list().await?;
client.tags().update(id, UpdateTagParams { name: Some("Question".into()), ..Default::default() }).await?;
let deleted = client.tags().delete(id).await?; // takes it off every post; posts stay
println!("{}", deleted["data"]["posts_untagged"]);

// Tag a post at create time: tag ids or names; a new name becomes a tag (at most 20 per post)
client.posts().create(CreatePostParams {
    content: "Would you post this?".into(),
    channels: Some(vec!["tiktok".into()]),
    tags: Some(vec!["Question".into(), 14.into()]),
    ..Default::default()
}).await?;

// Replace a post's tags; Some(None) sends null and removes them all. Works on published posts too.
client.posts().update("post_id", UpdatePostParams {
    tags: Some(Some(vec!["Question".into()])),
    ..Default::default()
}).await?;

// Up to 500 posts at once: add, remove, or set (replaces; Some(vec![]) removes every tag)
let result = client.tags().apply(ApplyTagsParams {
    post_ids: vec!["1024".into(), "1025".into()],
    add: Some(vec!["Question".into()]),
    ..Default::default()
}).await?;
println!("{} updated, not found: {}", result["data"]["updated"], result["data"]["not_found"]);

// Filter: a tag id or name, comma separated for any of several
client.posts().list(ListPostsParams { tag: Some("Question".into()), ..Default::default() }).await?;
client.analytics().overview(AnalyticsOverviewParams {
    period: Some("30d".into()),
    tag: Some("Question".into()),
    ..Default::default()
}).await?;

// Compare tags against your normal (default: the last 90 days)
let compared = client.analytics().tags(TagAnalyticsParams {
    start_date: Some("2026-08-01".into()),
    platform: Some("tiktok".into()),
    ..Default::default()
}).await?;
for row in compared["data"]["tags"].as_array().unwrap() {
    println!("{}: {} vs normal (enough posts: {})", row["tag"]["name"], row["vs_normal"], row["enough_posts"]);
}
```

`vs_normal` is a fraction (0.38 = 38% above your normal): each post is measured against the median impressions of its own channel in the period. Read it as a hint only when `enough_posts` is false. Every post carries `tags` and `trial_reel` (`{ "strategy": "MANUAL" | "SS_PERFORMANCE" }` on an Instagram Trial Reel, else `null`), and so does every `analytics().posts()` entry.

Errors: `404 tag_not_found`; `409 tag_exists` (the existing tag is in the error body under `error.tag`), `tag_limit_reached` (200 tags per workspace); `400 invalid_tag_name` (1 to 40 characters), `invalid_tag_color`, `invalid_tags`, `invalid_post_ids`, `too_many_posts`, `too_many_tags`.

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

`status` is `active` or `needs_reconnect` (the token was revoked or expired; Bluesky also when its sign-in could not be renewed). Posts to a `needs_reconnect` account fail until it is reconnected in the OmniSocials app. OmniSocials Connect customer workspaces can also show `pick_pending` (Facebook, Connect customers only: no Page chosen yet).

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

The response is not wrapped in `data`. `platform`, `timezone`, `basis`, `grid` and `recommendations` sit at the top level.

```rust
use omnisocials::BestTimesParams;

let best = client.analytics().best_times(BestTimesParams {
    platform: "instagram".into(),
    timezone: Some("Europe/Amsterdam".into()),
}).await?;
for slot in best["recommendations"].as_array().into_iter().flatten() {
    println!("{} {} {}", slot["day"], slot["time"], slot["score"]);
}
```

## Inbox

Read and reply to social conversations, DMs, comments, and mentions, across Instagram, Facebook, LinkedIn, TikTok (video comments only), YouTube (video comments only), X, and Threads. TikTok and YouTube replies are comments only; TikTok replies are capped at 150 characters. Threads conversations are `type` `"comment"` (replies people leave on your Threads posts; conversation ids look like `threads_comment_<rootPostId>`) and `"mention"` (`threads_mention_<postId>`); there are no Threads DMs, and a reply publishes as a native Threads reply. The Threads inbox needs a Threads connection with the reply permission (a connection without it fails with `Error::Auth`, status 401, code `reauth_required`; reconnect Threads to fix it, an account connected before 2026-09-14 needs this once).

```rust
use omnisocials::{ListConversationsParams, ReplyParams};

let conversations = client.inbox().list_conversations(ListConversationsParams {
    platform: Some("instagram".into()),
    unread: Some(true),
    ..Default::default()
}).await?;
let conversation_id = conversations["data"][0]["conversation_id"].as_str().unwrap();

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

// Like (or unlike) a comment as the connected Page or account: Facebook,
// LinkedIn Page and TikTok comments. The comment comes back with `liked` set.
let liked = client.inbox().like(message_id, true).await?; // like
client.inbox().like(message_id, false).await?;            // unlike
println!("{}", liked["data"]["liked"]); // true

// Delete a comment outright, or one of your own replies (Facebook,
// Instagram, TikTok; YouTube: hide instead). Replies under it go with it;
// their ids come back as `removed_reply_ids`.
client.inbox().delete_message(message_id).await?;
```

### Work queue: what needs an answer

`inbox().next()` hands out the next conversation that still needs a reply (the customer's latest DM with no reply after it, or an unreplied comment/mention that is not hidden), with the whole thread and the post it belongs to (`post.url`, `post.media_type`), so a reply can be drafted from one call. DMs that can still be answered come first, then Instagram/Facebook DMs whose 24-hour window has closed (`reply_window.open` is `false`: answer those from the native app or mark them read), then comments and mentions, oldest first. Replies typed in the native apps count as answers. Only unread items are served by default, so `mark_read` is the durable way to skip one; `exclude` skips conversation ids for the current session only. Always set `message_id` to the served `message.id` on `reply` for comment threads: every comment on a post shares one conversation, and without it the reply goes under the newest comment on the post. Set `include_next: Some(true)` on `reply` to get the following item in the same response. `ListConversationsParams { unanswered: Some(true), .. }` gives the same set as a plain list.

```rust
use omnisocials::NextUnansweredParams;

let mut next = client.inbox().next(NextUnansweredParams {
    platform: Some("instagram".into()),
    ..Default::default()
}).await?;
while !next["data"].is_null() {
    let message = &next["data"]["message"];
    println!("{} left. {}: {}", next["remaining"], message["sender"]["username"], message["text"]);

    if next["data"]["reply_window"]["open"] == false {
        // An Instagram/Facebook DM past Meta's 24-hour window: reply would fail
        // with 422 outside_messaging_window. Answer it in the app, or skip it.
        client.inbox().mark_read(message["conversation_id"].as_str().unwrap()).await?;
        next = client.inbox().next(NextUnansweredParams {
            platform: Some("instagram".into()),
            ..Default::default()
        }).await?;
        continue;
    }

    let reply = client.inbox().reply(message["conversation_id"].as_str().unwrap(), ReplyParams {
        text: "Thanks! DM sent.".into(),
        message_id: message["id"].as_str().map(str::to_owned), // the comment being answered, not the newest one
        include_next: Some(true),
        ..Default::default()
    }).await?;
    next = serde_json::json!({ "data": reply["next"], "remaining": reply["remaining"] });
}
```

`list_conversations` and `get_messages` are cursor-paginated: read `pagination.next_cursor` from the response and pass it back as `cursor` to fetch the next page, and stop once `pagination.has_more` is `false`.

### X DM replies use credits

Replying to an X DM costs 2 prepaid credits per send, debited before the send and automatically refunded if the send fails. If the balance can't cover it, `reply` fails with `Error::Api` (status 402) and code `insufficient_credits`. If the workspace's X inbox was auto-suspended for hitting a zero balance, it fails with code `x_inbox_suspended` instead; a top-up turns the inbox back on by itself (DMs that arrived while suspended are not recovered).

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
        eprintln!("X inbox suspended until a top-up");
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

## Pinterest product tags

Tag products on a Pin so people can shop the items in the image. `client.pinterest().list_products(...)` returns the product Pins of the connected Pinterest account; pass their `pin_id` values (max 24) as `product_tags` in the `pinterest` options of the post. Only product Pins of your own account can be tagged; products of other merchants cannot. The tags are added right after the Pin is published. A product that Pinterest refuses never fails the post: the outcome is on the post as `pinterest.product_tags_result` (`requested`, `tagged`, `skipped`, `error`).

```rust
use omnisocials::ListPinterestProductsParams;

let result = client.pinterest().list_products(ListPinterestProductsParams::default()).await?;

if let Some(error) = result.get("error") {
    // HTTP 200 without "products": pinterest_not_connected,
    // pinterest_catalog_access_required or platform_error
    eprintln!("{}: {}", error["code"], error["message"]);
} else {
    let product_tags: Vec<&str> = result["products"]
        .as_array()
        .map(|products| products.iter().filter_map(|product| product["pin_id"].as_str()).take(3).collect())
        .unwrap_or_default();

    client.posts().create(CreatePostParams {
        content: "Our summer picks".into(),
        channels: Some(vec!["pinterest".into()]),
        media_urls: Some(vec!["https://example.com/summer-look.jpg"].into()),
        scheduled_at: Some("2026-08-01T09:00:00Z".into()),
        pinterest: Some(json!({
            "board_id": "1234567890",
            "title": "Summer picks",
            "product_tags": product_tags,
        })),
        ..Default::default()
    }).await?;
}
```

Without `source` the list reads the Pinterest catalog (with `price`, `currency`, `availability` and `item_id`) when the connection has catalog access, else the account's own Pins. Catalog access is given one time in the OmniSocials composer: Pinterest options, Add products, Connect catalog. `source: Some("pins".into())` scans up to 250 Pins per call, so `products` can be empty while `bookmark` is set; call again with the bookmark in `ListPinterestProductsParams.bookmark`. To check one Pin id or Pin link before you post, call `client.pinterest().validate_product("813744226420795884").await?`. On `posts().update` the `pinterest` value replaces the stored one, so leave `product_tags` out to remove the tags.

## Ads

Boost a published post or create a new ad on Meta, Pinterest, TikTok, LinkedIn, X and Google. Boosts and new ads are drafts: your code writes the draft, and the person who pays sees the amount and starts the ad in OmniSocials from the draft's `review_url`. A key starts and raises spend itself only with the opt-in **Start ads** switch (scope `ads:spend`). Without it `ads().activate`, `ads().activate_multi`, `ads().resume`, `ads().drafts().launch`, a budget raise or any change of a campaign that reached the network through `ads().update`, and the calls of the groups below that switch something on or raise a budget answer `403 start_in_app` (`Error::PermissionDenied`), and the error message names the page where the person does it. Google (`api_draft_only` of `ads().capabilities()`) always stops at the draft, also with `ads:spend`. `ads().pause` and `ads().delete` stay open, except on the networks in `api_stop_in_app` (Google), where a campaign that reached the network also answers `403 start_in_app`. Only a paused campaign resumes (`409 not_paused`). The calls that start or raise spend repeat the amount (`confirm_budget_amount`) and are not retried after a 5xx or a connection error, only after a 429.

The `ads:read` and `ads:write` scopes are opt-in: create the API key with **Ads** switched on (Settings, API). A key without them gets `Error::PermissionDenied` with code `insufficient_scope`. `ads:spend` is the **Start ads** switch under it, offered only to a person with `manage_ads`. Ad accounts are connected and picked in the OmniSocials dashboard. Budgets are in major units (10 = 10 EUR) in the ad account's currency: a draft whose `budget.currency` differs from the attached ad account's currency answers `400 currency_mismatch`. Ads errors may carry `param`, the field at fault: read it from the error's `body` (`body["error"]["param"]`). Campaign and ad account ids are strings (`"12"`), as the API sends them and like every other id in this crate. A campaign carries `createdAt` and `updatedAt` in camelCase; an ad account carries `created_at`. In `overview()`, `totals.spend` is `null` when the campaigns use more than one currency: read `totals_by_currency`.

### Create an ad (a draft the person starts)

```rust
use omnisocials::{
    AdBudget, AdDraftAd, AdDraftParams, AdDraftSchedule, AdDraftTargeting, AdMediaFile,
};

let caps = client.ads().capabilities().await?;
println!("{} {}", caps["data"]["create_networks"], caps["data"]["network_states"]);

let draft = client.ads().drafts().create(AdDraftParams {
    name: Some("Autumn sale".into()),
    objective: "traffic".into(),
    platforms: vec!["meta".into(), "linkedin".into()],
    budget: AdBudget { amount: 10.0, currency: "EUR".into(), ..Default::default() },
    schedule: Some(AdDraftSchedule { duration_days: Some(7) }),
    targeting: AdDraftTargeting {
        countries: vec!["NL".into(), "BE".into()],
        automatic_audience: Some(true),
        ..Default::default()
    },
    ads: vec![AdDraftAd {
        media: vec![AdMediaFile {
            url: "https://example.com/sale.jpg".into(),
            r#type: "image".into(),
            width: Some(1080),
            height: Some(1350),
            ..Default::default()
        }],
        primary_text: "30% off everything this week".into(),
        headline: Some("Autumn sale".into()),
        cta: Some("shop_now".into()),
        link: "https://example.com/sale".into(),
        ..Default::default()
    }],
}).await?;

// A draft spends nothing. The person checks it and starts it in OmniSocials:
println!("Check and start your ad: {}", draft["data"]["review_url"]);
```

With a key that has `ads:spend`, `client.ads().drafts().launch(group_id, LaunchAdDraftParams { confirm_budget_amount: Some(10.0), ..Default::default() })` (`group_id` from `draft["data"]["group_id"]`) starts the draft after the person agreed to the amount; Google is skipped with code `start_in_app`. Without `ads:spend` the call answers `403 start_in_app`.

A draft needs no ad account; `client.ads().drafts().attach_account(group_id, AttachAdDraftAccountParams { platform, ad_account_id })` sets one, and `campaigns[].readiness` says what still stops a network. `drafts().update` replaces the draft with the full body (`PUT`). On Meta an ad can carry a second upright 9:16 file for Stories and Reels: `placement_media: Some(AdPlacementMedia { stories_reels: Some(AdMediaFile { .. }) })`.

### Boost a post

```rust
use omnisocials::{AdBudget, BoostPostParams};
use serde_json::json;

let accounts = client.ads().list_accounts().await?;
let meta = accounts["data"]
    .as_array()
    .unwrap()
    .iter()
    .find(|a| a["platform"] == "meta" && a["is_selected"] == true)
    .expect("no Meta ad account picked");
// Ad ids arrive as strings, e.g. "12".
let account_id = meta["id"].as_str().unwrap();
let currency = meta["account_info"]["currency"].as_str().unwrap_or("EUR");

let boost = client.ads().boost(BoostPostParams {
    post_id: "12345".into(),
    ad_account_id: account_id.into(),
    budget: AdBudget { r#type: Some("daily".into()), amount: 5.0, currency: currency.into() },
    targeting: Some(json!({ "countries": ["NL"] })),
    ..Default::default()
}).await?;

// The boost is a draft: the person starts it in OmniSocials.
println!("{}", boost["data"]["status"]); // "draft"
```

`client.ads().boost_multi(...)` makes one draft boost per ad account (up to 10, the same budget each). `client.ads().suggestions()` lists posts that do better than your median, `client.ads().overview()` the totals, and `list`, `get`, `pause` and `delete` manage campaigns. Keyword ideas through the connected Google Ads account: `client.ads().research_keywords(KeywordResearchParams { keywords: Some(vec!["running shoes".into()]), country: Some("NL".into()), ..Default::default() })` and `client.ads().keyword_metrics(KeywordMetricsParams { keywords: vec![...], ..Default::default() })`.


### The rest of the Ads API

Each group hangs off `client.ads()`. A network without a feature answers `400 not_supported_on_network`; `capabilities()` lists what each network offers (`data.features`). Ids are strings like everywhere in this crate. A query is anything that serializes to a JSON object (`json!({"from": "2026-10-01", "limit": 50})`; a list becomes a comma list, `null` is left out; pass `()` for none) and a body anything that serializes to JSON; results are `serde_json::Value`.

| Accessor | What it covers |
|----------|----------------|
| `objects()` | the live tree of an ad account (`tree`), one campaign, ad set or ad (`get`, `update`, `set_status`, `duplicate`), the import of campaigns made outside OmniSocials (`import_status`, `start_import`, `list_external`, `get_external`) |
| `campaign_ads()` | the ads of a campaign: `list`, `add` (created paused), `get`, `set_status`, `review` |
| `targeting()` | `search`, `browse`, `reach_estimate`, `bid_suggestions` |
| `audiences()` | customer lists, lookalikes and company lists: `list`, `create`, `get`, `update`, `delete`, `add_members`, `remove_members`, `add_companies` (raw emails and phones are hashed before they leave) |
| `lead_forms()`, `leads()` | lead forms (`list`, `create`, `get`, `update`, `delete`, `send_test_lead`) and their leads (`list`, `counts`, `update`) |
| `conversions()` | `destinations`, `goals`, `create_custom`, `send_events` |
| `insights()`, `reports()` | `query`, `campaign`, `ad`, `timeline`, `performance`, `alerts`; background reports (`list`, `start`, `get`) |
| `rules()` | automated rules: `list`, `create`, `templates`, `preview`, `activity`, `undo`, `get`, `update`, `delete`, `preview_saved`, `runs` |
| `results()` | your own numbers per campaign and day: `list`, `upload`, `totals`, `metrics`, `put_metric` |
| `media()`, `previews()`, `partnership()`, `comments()` | the network's media library, previews drawn by the network (`create`, `for_ad`), Meta partnership ads (`list_content`, `list_permissions`, `set_permission`), comments on ads |
| `accounts()` | one ad account: `identities`, `health`, `finance`, `update_settings`, `list_users`, `add_user`, `remove_user`, `activity`, `businesses`, `dsa`, `hierarchy` |
| `reach_frequency()`, `studies()`, `budget_schedules()` | Meta reach and frequency, A/B tests, planned budget increases |
| `google()` | Google reports: `search_terms`, `recommendations`, `keywords`, `assets`, `shared_budgets`, `bid_strategies`, `ad_schedule`, `labels`, `local_services_leads` |

On campaigns: `client.ads().duplicate(id, ())` (a new draft) and `client.ads().bulk_status(&ids, "paused", None)`. `UpdateAdCampaignParams` takes `settings` (while the campaign is a draft), `AdDraftPlatform::Detailed` takes `settings` and `AdDraftTargeting` takes `networks` (the network's own targeting).

```rust
use serde_json::json;

// Search an interest, then target it on Meta in a draft.
let found = client
    .ads()
    .targeting()
    .search(json!({"ad_account_id": 51, "type": "interest", "q": "yoga"}))
    .await?;
let item = &found["data"]["items"][0];
let targeting = omnisocials::AdDraftTargeting {
    countries: vec!["NL".into()],
    networks: Some(json!({"meta": {"include": {"interest": [{"id": item["id"], "name": item["name"]}]}}})),
    ..Default::default()
};

// Pause one ad set made outside OmniSocials (no amount needed to pause).
client.ads().objects().set_status("51", "ad_set", "23851", "paused", None).await?;
```

The same money rule holds here: switching anything on (`objects().set_status`, `campaign_ads().set_status`, `bulk_status` with `"active"`), a higher budget (`objects().update`), a rule that can raise spend (`rules().create` and `rules().update` with `confirm_max_daily_budget`, `rules().undo`), a higher spend cap (`accounts().update_settings`), a reach and frequency reservation (`reach_frequency().reserve`) and a planned budget increase (`budget_schedules().create`) each need the amount from the person's explicit yes, and none of them is retried after a 5xx or a connection error. These calls, and the other changes of a campaign on the network (`objects().duplicate`, `campaign_ads().add`, `studies().create`), need a key with `ads:spend`: without it, and on Google always, they answer `403 start_in_app` with the page where a person does it in the app. A rule that switches on or raises a budget needs `ads:spend` too (else `403 rule_raises_spend`); a rule that notifies, pauses or lowers a budget works with `ads:write`, and no rule acts on Google. Pausing, deleting and archiving stay open, except on Google.

## Webhooks

Events: `post.scheduled`, `post.published`, `post.failed`, `post.approved` (the last step of a post's approval workflow is approved) and `post.rejected` (an approver rejects the post; it will not publish). The two approval events carry `data.approval` with `status`, `decided_by` (the approver's user id) and `reason` (`null` on `post.approved`), and an empty `data.targets`. `ad.status_changed` fires when an ad campaign changes status (for example `pending_review` to `active` or `rejected`); its `data` is the campaign, not a post: `campaign_id`, `workspace_id`, `name`, `platform`, `campaign_type`, `status`, `previous_status`, `external_campaign_id` and `budget`. More ad events: `ad.created`, `ad.updated`, `ad.review`, `ad.delivery_issue`, `campaign.created`, `campaign.updated`, `campaign.budget` (these carry the campaign plus `details`), `lead.received`, `ads.sync_failed`, `ads.sync_recovered`, `ads.initial_sync_completed` and `ad_rule.triggered`. Subscribing to `ad.status_changed` or any other ads event needs the `ads:read` scope on the key and `access_ads` for its person (`403 insufficient_scope` / `insufficient_permission`).

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
println!("{}", rotated["data"]["secret"]); // the old secret keeps signing for 24 hours
client.webhooks().delete(id).await?;
```

Every webhook carries `workspace_id`. `list` and `get` also return the organisation-wide webhooks of the key's organisation (`workspace_id` `null`, made in the OmniSocials dashboard; they receive the events of every workspace), and `update`, `delete` and `rotate_secret` work on them too. Webhooks you create through the API belong to the key's workspace. When the person behind a webhook loses access to the organisation or workspace, OmniSocials switches the webhook off: `is_active` is `false` and `last_failure.error` says why. `update` with `is_active: Some(true)` switches it on again.

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
                Some("post.rejected") => {
                    eprintln!("Rejected: {} {}", event["data"]["post_id"], event["data"]["approval"]["reason"]);
                }
                Some("ad.status_changed") => {
                    // The data is the campaign, not a post.
                    println!("Ad: {} {}", event["data"]["campaign_id"], event["data"]["status"]);
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

### Rotate the secret without missing deliveries

For 24 hours after `rotate_secret`, every delivery carries two headers: `X-OmniSocials-Signature`, signed with the new secret, and `X-OmniSocials-Signature-Previous`, signed with the old secret (same format, same timestamp and body). Verify the first with the new secret and, when that fails, the second with the old one:

```rust
use omnisocials::webhooks::{verify_signature, DEFAULT_TOLERANCE_SECS};

fn verify_delivery(
    body: &[u8],
    headers: &HeaderMap,
    new_secret: &str,
    old_secret: &str,
) -> Result<serde_json::Value, omnisocials::Error> {
    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok()).unwrap_or("");
    verify_signature(body, header("x-omnisocials-signature"), new_secret, DEFAULT_TOLERANCE_SECS).or_else(|err| {
        match header("x-omnisocials-signature-previous") {
            "" => Err(err),
            previous => verify_signature(body, previous, old_secret, DEFAULT_TOLERANCE_SECS),
        }
    })
}
```

Rotating again within the 24 hours replaces the previous secret with the one you just rotated out.

## Limits and feature requests

`client.limits().list(...)` lists what OmniSocials cannot do, why (`says`) and what to do instead (`instead`): platform rules, features not built yet, features that work in the OmniSocials app only. Read it before you tell a person that something is not possible. When the person still wants a `requestable` entry, send a feature request with its `limit_id`. The request is stored for the person who owns the key; they see it in OmniSocials (Settings, Request a feature) and get an email when its status changes. Works with every key, no scope needed.

```rust
use omnisocials::{CreateFeatureRequestParams, Error, ListLimitsParams};

let limits = client
    .limits()
    .list(ListLimitsParams { platform: Some("telegram".into()), ..Default::default() }) // area: "posts", "inbox", ...
    .await?;
for limit in limits["data"].as_array().into_iter().flatten() {
    println!("{}: {} requestable={}", limit["id"], limit["says"], limit["requestable"]);
}

match client
    .feature_requests()
    .create(CreateFeatureRequestParams {
        summary: "Post our daily update to the Announcements topic of our Telegram group, not to the whole group.".into(),
        platform: Some("telegram".into()),
        limit_id: Some("telegram.forum_topics".into()),
        ..Default::default()
    })
    .await
{
    // "created" (new), "voted" (added a vote to your organisation's open request)
    // or "already_requested" (you asked before; data is that request)
    Ok(result) => println!("{} {} {}", result["outcome"], result["data"]["id"], result["data"]["status"]),
    Err(Error::Api { code, body, .. }) if code.as_deref() == Some("not_requestable") => {
        // A platform rule or a choice of OmniSocials: nothing was stored.
        let body = body.unwrap_or_default();
        println!("{} {}", body["error"]["limit"]["says"], body["error"]["limit"]["instead"]);
    }
    Err(err) => return Err(err),
}

let mine = client.feature_requests().list().await?; // newest first, at most 50
let first_id = mine["data"][0]["id"].as_i64().unwrap_or_default();
let one = client.feature_requests().get(first_id).await?; // status, team_answer, team_replies
```

A limit is `{"id", "kind", "kind_label", "area", "platforms", "title", "says", "instead", "requestable"}`; `kind` is `platform`, `by_design`, `not_built`, `beta`, `dashboard_only` or `needs_setup`, and `requestable` is false for `platform` and `by_design`. `limits().list()` also returns `kinds` (a label per kind). A request carries `status` (`open`, `in_progress`, `completed`, `declined`), `votes_count`, `team_answer` and `team_replies`. `summary` is 10 to 1000 characters (what the person wants to do and why), `title` at most 120 (made from the summary when `None`); `area` is one of posts, media, inbox, analytics, automations, accounts, ads, team, other. Errors: `400 invalid_summary` / `invalid_title` / `invalid_platform` / `invalid_area` / `unknown_limit` (`Error::Validation`), `409 not_requestable` (`Error::Api`, the entry is in `error.limit` of `body`), `429 too_many_requests` (`Error::RateLimit`, 10 an hour per person), and `404 not_found` on `get`.

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

Each API key carries scopes: `posts:read`, `posts:write`, `media:write`, `accounts:read`, `analytics:read`, `webhooks:manage`, plus the opt-in `automations:read` / `automations:write` for comment-to-DM automations. A call with a missing scope returns `Error::PermissionDenied` with code `insufficient_scope`.

## Documentation

Full API reference and guides: [https://docs.omnisocials.com](https://docs.omnisocials.com)

## License

MIT
