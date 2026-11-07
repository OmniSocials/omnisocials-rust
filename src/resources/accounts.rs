use serde_json::Value;

use crate::client::{encode_path_segment, Client};
use crate::error::Error;

/// `client.accounts()`: connected social accounts.
#[derive(Debug, Clone, Copy)]
pub struct Accounts<'a> {
    pub(crate) client: &'a Client,
}

impl Accounts<'_> {
    /// `GET /accounts` - the workspace's connected social accounts. Each
    /// account carries a `status`: `"active"`, `"needs_reconnect"` when the
    /// platform has revoked or expired its OAuth token (Bluesky: the sign-in
    /// could not be renewed), or `"pick_pending"` (Facebook, OmniSocials
    /// Connect customers only: the Facebook login manages several Pages and
    /// none is chosen yet; posts fail until the Page is chosen by connecting
    /// Facebook again and picking the Page). The booleans `needs_reconnect` and `pick_pending`
    /// mirror it. When `pick_pending` is `true`, `username`, `display_name`
    /// and `profile_picture` name the person who signed in, not a Page.
    pub async fn list(&self) -> Result<Value, Error> {
        self.client.get("/accounts", Vec::new()).await
    }

    /// `GET /accounts/:id` - a single connected account.
    pub async fn get(&self, id: &str) -> Result<Value, Error> {
        self.client
            .get(&format!("/accounts/{}", encode_path_segment(id)), Vec::new())
            .await
    }
}
