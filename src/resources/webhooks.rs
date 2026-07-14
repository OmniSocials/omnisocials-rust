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
    /// `GET /webhooks` - list the workspace's webhook endpoints.
    pub async fn list(&self) -> Result<Value, Error> {
        self.client.get("/webhooks", Vec::new()).await
    }

    /// `GET /webhooks/:id` - fetch a single webhook.
    pub async fn get(&self, id: &str) -> Result<Value, Error> {
        self.client
            .get(&format!("/webhooks/{}", encode_path_segment(id)), Vec::new())
            .await
    }

    /// `POST /webhooks` - register an endpoint for event deliveries
    /// (`post.scheduled`, `post.published`, `post.failed`). The response
    /// includes the signing `secret`; save it, it is only shown once.
    pub async fn create(&self, params: CreateWebhookParams) -> Result<Value, Error> {
        self.client.post_json("/webhooks", &params).await
    }

    /// `PATCH /webhooks/:id` - change the URL, events, or active state.
    pub async fn update(&self, id: &str, params: UpdateWebhookParams) -> Result<Value, Error> {
        self.client
            .patch_json(&format!("/webhooks/{}", encode_path_segment(id)), &params)
            .await
    }

    /// `DELETE /webhooks/:id` - delete a webhook. Resolves to
    /// [`Value::Null`] (204).
    pub async fn delete(&self, id: &str) -> Result<Value, Error> {
        self.client
            .delete(&format!("/webhooks/{}", encode_path_segment(id)))
            .await
    }

    /// `POST /webhooks/:id/rotate-secret` - generate a new signing secret.
    /// Save the returned secret; it is only shown once and the old one stops
    /// working immediately.
    pub async fn rotate_secret(&self, id: &str) -> Result<Value, Error> {
        self.client
            .post_empty(&format!("/webhooks/{}/rotate-secret", encode_path_segment(id)))
            .await
    }
}
