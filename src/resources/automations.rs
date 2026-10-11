use serde_json::Value;

use crate::client::{encode_path_segment, Client};
use crate::error::Error;
use crate::types::{
    CreateAutomationParams, ListAutomationEventsParams, ListAutomationsParams,
    UpdateAutomationParams,
};

/// `client.automations()`: comment-to-DM automations. An automation answers
/// a keyword in an Instagram comment (or a DM, or a story reply) with a DM,
/// once per person, with an optional public reply under the comment.
/// Instagram rules: one private reply per comment, within 7 days;
/// OmniSocials caps sends at 180 DMs an hour per account. The scopes are
/// opt-in on the API key: `automations:read` for `list`, `get` and
/// `list_events`, `automations:write` for `create`, `update` and `delete`
/// (and for [`crate::CreatePostParams::comment_dm_automation`]).
#[derive(Debug, Clone, Copy)]
pub struct Automations<'a> {
    pub(crate) client: &'a Client,
}

impl Automations<'_> {
    /// `GET /automations` - list the workspace's comment-to-DM automations,
    /// optionally by `status` (`"active"`, `"paused"`, `"completed"`) or by
    /// the OmniSocials `post_id` they are bound to. Requires the
    /// `automations:read` scope (opt-in on the API key).
    pub async fn list(&self, params: ListAutomationsParams) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(status) = params.status {
            query.push(("status", status));
        }
        if let Some(post_id) = params.post_id {
            query.push(("post_id", post_id));
        }
        self.client.get("/automations", query).await
    }

    /// `GET /automations/:id` - fetch a single automation. Requires the
    /// `automations:read` scope (opt-in on the API key).
    pub async fn get(&self, id: &str) -> Result<Value, Error> {
        self.client
            .get(&format!("/automations/{}", encode_path_segment(id)), Vec::new())
            .await
    }

    /// `POST /automations` - create a comment-to-DM automation. Bind it to
    /// one post with `post_id` (an OmniSocials post that goes to Instagram,
    /// scheduled or live; one automation per post) or `instagram_media_id`
    /// (a post published outside OmniSocials), not both; a `trigger` with
    /// `"scope": "next_post"` or `"any"` needs neither. `actions` is
    /// required (`dm.text`, max 1000 characters). Requires the
    /// `automations:write` scope (opt-in on the API key). Errors:
    /// `400 validation_error`, `400 post_not_instagram`,
    /// `400 post_required`, `404 post_not_found`,
    /// `404 instagram_media_not_found`, `409 instagram_not_connected`,
    /// `409 instagram_needs_reauth`, `409 automation_exists` (the error body
    /// also carries `automation_id`).
    pub async fn create(&self, params: CreateAutomationParams) -> Result<Value, Error> {
        self.client.post_json("/automations", &params).await
    }

    /// `PATCH /automations/:id` - partial update. Each section (`trigger`,
    /// `actions`, `limits`) is merged into the stored one; keys you omit
    /// keep their value: new `trigger.keywords` keep the kinds, the scope
    /// and the match; a new `actions.dm.text` keeps the public reply;
    /// `limits.stop_at` keeps `max_sends`. `public_reply.enabled: false` or
    /// `variations: []` turns the public reply off; `limits.max_sends:
    /// null` removes that limit. `status: "paused"` pauses and `"active"`
    /// resumes; another `post_id` / `instagram_media_id` moves the
    /// automation to that post. A `completed` automation starts again when
    /// its limits or trigger change. Requires the `automations:write`
    /// scope. `400 no_fields` when the body is empty.
    pub async fn update(&self, id: &str, params: UpdateAutomationParams) -> Result<Value, Error> {
        self.client
            .patch_json(&format!("/automations/{}", encode_path_segment(id)), &params)
            .await
    }

    /// `DELETE /automations/:id` - delete an automation. Resolves to
    /// [`Value::Null`] (204). Requires the `automations:write` scope.
    pub async fn delete(&self, id: &str) -> Result<Value, Error> {
        self.client
            .delete(&format!("/automations/{}", encode_path_segment(id)))
            .await
    }

    /// `GET /automations/:id/events` - the automation's activity: one event
    /// per person it answered, skipped or failed on. Filter by `outcome`
    /// (`"sent"`, `"skipped"`, `"failed"`, `"pending"`); `limit` is 1 to
    /// 100 (default 25), `offset` skips. Requires the `automations:read`
    /// scope.
    ///
    /// The response is not the plain list envelope: `{"data": [...],
    /// "counts": {"sent": 12, "skipped": 3}, "pagination": {...}}`, where
    /// `counts` is the number of events per outcome (an outcome with no
    /// events is left out). Each event carries `kind` (`comment`, `dm`,
    /// `story_reply`), `outcome`, `reason` / `reason_label`, `person`
    /// (`id`, `username`), `trigger_id`, `trigger_text`, `message` (the DM
    /// sent), `public_reply`, `platform_message_id`, `created_at` and
    /// `sent_at`.
    pub async fn list_events(
        &self,
        id: &str,
        params: ListAutomationEventsParams,
    ) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(outcome) = params.outcome {
            query.push(("outcome", outcome));
        }
        if let Some(limit) = params.limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(offset) = params.offset {
            query.push(("offset", offset.to_string()));
        }
        self.client
            .get(&format!("/automations/{}/events", encode_path_segment(id)), query)
            .await
    }
}
