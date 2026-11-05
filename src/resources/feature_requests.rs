use serde_json::Value;

use crate::client::Client;
use crate::error::Error;
use crate::feature_request_params::CreateFeatureRequestParams;

/// `client.feature_requests()`: requests to the OmniSocials team, for the
/// person who owns the key (their name and email are stored with it so the
/// team can answer). The person sees them in OmniSocials (Settings, Request
/// a feature) and gets an email when the status changes. No scope needed.
///
/// A request is `{"id", "title", "summary", "status" (open, in_progress,
/// completed, declined), "team_answer", "team_replies": [{"message",
/// "created_at"}], "votes_count", "limit_id", "platform", "area", "source",
/// "created_at", "updated_at", "completed_at"}`.
///
/// Error codes: 400 `invalid_summary`, `invalid_title`, `invalid_platform`,
/// `invalid_area`, `unknown_limit`, `invalid_id`; 404 `not_found`; 409
/// `not_requestable` (an [`Error::Api`]: the `limit_id` is a platform rule
/// or a choice of OmniSocials, the entry is in the error body under
/// `error.limit` and nothing was stored); 429 `too_many_requests` (10 an
/// hour per person).
#[derive(Debug, Clone, Copy)]
pub struct FeatureRequests<'a> {
    pub(crate) client: &'a Client,
}

impl FeatureRequests<'_> {
    /// `POST /feature-requests` - send a request. Send it only when the
    /// person asked for it. Returns `{"outcome", "data"}`: `outcome` is
    /// `"created"` (a new request), `"voted"` (a vote added to the
    /// organisation's open request for the same gap) or
    /// `"already_requested"` (this person already has an open request for
    /// it; `data` is that request).
    pub async fn create(&self, params: CreateFeatureRequestParams) -> Result<Value, Error> {
        self.client.post_json("/feature-requests", &params).await
    }

    /// `GET /feature-requests` - the person's own requests, newest first,
    /// at most 50: `{"data": [FeatureRequest]}`.
    pub async fn list(&self) -> Result<Value, Error> {
        self.client.get("/feature-requests", Vec::new()).await
    }

    /// `GET /feature-requests/:id` - one request with its status and the
    /// team's replies.
    pub async fn get(&self, id: i64) -> Result<Value, Error> {
        self.client.get(&format!("/feature-requests/{id}"), Vec::new()).await
    }
}
