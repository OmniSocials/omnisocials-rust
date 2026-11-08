use serde_json::Value;

use crate::client::{encode_path_segment, Client};
use crate::error::Error;
use crate::types::{CreateWebhookParams, UpdateWebhookParams};

/// `client.webhooks()`: webhook endpoint management. To verify incoming
/// deliveries, use [`crate::webhooks::verify_signature`].
#[derive(Debug, Clone, Copy)]
pub struct Webhooks<'a> {
    pub(crate) client: &'a Client,
}

impl Webhooks<'_> {
    /// `GET /webhooks` - list the webhooks of the key's workspace and the
    /// organisation-wide webhooks of its organisation. Every webhook carries
    /// `workspace_id`: `null` for an organisation-wide webhook (made in the
    /// OmniSocials dashboard), which receives the events of every workspace
    /// of the organisation. A webhook whose creator lost access to the
    /// organisation or workspace is switched off
    /// automatically: `is_active` is `false` and `last_failure.error` says
    /// why.
    pub async fn list(&self) -> Result<Value, Error> {
        self.client.get("/webhooks", Vec::new()).await
    }

    /// `GET /webhooks/:id` - fetch a webhook of the key's workspace, or an
    /// organisation-wide webhook of its organisation (`workspace_id` `null`).
    pub async fn get(&self, id: &str) -> Result<Value, Error> {
        self.client
            .get(&format!("/webhooks/{}", encode_path_segment(id)), Vec::new())
            .await
    }

    /// `POST /webhooks` - register an endpoint for event deliveries
    /// (`post.scheduled`, `post.published`, `post.failed`, `post.approved`,
    /// `post.rejected`, and for ads `ad.status_changed`, `ad.created`,
    /// `ad.updated`, `ad.review`, `ad.delivery_issue`, `campaign.created`,
    /// `campaign.updated`, `campaign.budget`, `lead.received`,
    /// `ads.sync_failed`, `ads.sync_recovered`, `ads.initial_sync_completed`,
    /// `ad_rule.triggered`, and for X statistics `x_statistics.paused` and
    /// `x_statistics.resumed`, whose `data` is `{ workspace_id, account_id,
    /// reason }` with `reason` `insufficient_credits` or `credits_added`).
    /// The response includes the signing `secret`;
    /// save it, it is only shown once. An `ad.status_changed` delivery
    /// carries the campaign in `data` (`campaign_id`, `workspace_id`, `name`,
    /// `platform`, `campaign_type`, `status`, `previous_status`,
    /// `external_campaign_id`, `budget`), not a post; the other campaign
    /// events carry the campaign plus `details`, the rest their own `data`.
    /// Subscribing to any ads event needs the `ads:read` scope on the key
    /// and `access_ads` for its person (`403 insufficient_scope` /
    /// `insufficient_permission`).
    pub async fn create(&self, params: CreateWebhookParams) -> Result<Value, Error> {
        self.client.post_json("/webhooks", &params).await
    }

    /// `PATCH /webhooks/:id` - change the URL, events, or active state.
    /// Works on organisation-wide webhooks too. `is_active: Some(true)`
    /// switches on a webhook that was switched off (also an automatic
    /// switch-off); setting `url`, `events` or switching it on makes the
    /// key's creator the person behind the webhook. Setting `url` on an
    /// organisation-wide webhook (even its current one) moves it to the
    /// key's workspace: from then on it gets only that workspace's events,
    /// and the answer shows the new `workspace_id`. When the webhook has Ads
    /// events after the change, setting `url` or `events` or switching it on
    /// needs the `ads:read` scope and `access_ads` for the key's creator
    /// (403 otherwise, nothing changed).
    pub async fn update(&self, id: &str, params: UpdateWebhookParams) -> Result<Value, Error> {
        self.client
            .patch_json(&format!("/webhooks/{}", encode_path_segment(id)), &params)
            .await
    }

    /// `DELETE /webhooks/:id` - delete a webhook of the key's workspace or an
    /// organisation-wide webhook of its organisation. Resolves to
    /// [`Value::Null`] (204).
    pub async fn delete(&self, id: &str) -> Result<Value, Error> {
        self.client
            .delete(&format!("/webhooks/{}", encode_path_segment(id)))
            .await
    }

    /// `POST /webhooks/:id/rotate-secret` - generate a new signing secret.
    /// Save the returned secret; it is only shown once. The old secret keeps
    /// signing for 24 hours: every delivery then carries
    /// `X-OmniSocials-Signature` (new secret) and
    /// `X-OmniSocials-Signature-Previous` (old secret, same format, same
    /// timestamp and body). Verify the first with the new secret and, when
    /// that fails, the second with the old one (see
    /// [`crate::webhooks::verify_signature`]). Rotating again within the 24
    /// hours replaces the previous secret with the one just rotated out.
    /// Works on organisation-wide webhooks too.
    pub async fn rotate_secret(&self, id: &str) -> Result<Value, Error> {
        self.client
            .post_empty(&format!("/webhooks/{}/rotate-secret", encode_path_segment(id)))
            .await
    }
}
