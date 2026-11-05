use serde_json::Value;

use crate::client::Client;
use crate::error::Error;
use crate::feature_request_params::ListLimitsParams;

/// `client.limits()`: what OmniSocials cannot do, why, and what to do
/// instead (platform rules, features not built yet, features that work in
/// the OmniSocials app only). Read it before you tell a person that
/// something is not possible, so you can give the reason (`says`) and the
/// way around it (`instead`). When the person still wants a `requestable`
/// entry, send it with [`crate::resources::FeatureRequests::create`] and its
/// `limit_id`. No scope needed.
///
/// Error codes: 400 `invalid_platform`, `invalid_area`.
#[derive(Debug, Clone, Copy)]
pub struct Limits<'a> {
    pub(crate) client: &'a Client,
}

impl Limits<'_> {
    /// `GET /limits` - `{"data": [Limit], "kinds": {kind: label}}`. A limit
    /// is `{"id", "kind", "kind_label", "area", "platforms", "title", "says",
    /// "instead", "requestable"}`; `kind` is `platform`, `by_design`,
    /// `not_built`, `beta`, `dashboard_only` or `needs_setup`, and
    /// `requestable` is false for `platform` and `by_design`.
    pub async fn list(&self, params: ListLimitsParams) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(platform) = params.platform {
            query.push(("platform", platform));
        }
        if let Some(area) = params.area {
            query.push(("area", area));
        }
        self.client.get("/limits", query).await
    }
}
